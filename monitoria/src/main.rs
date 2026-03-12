use clap::{Parser, Subcommand};
use rusqlite::{params, Connection, Result as SqlResult};
use chrono::{Datelike, NaiveDate};
use rust_xlsxwriter::{Workbook, Format, Color};
use std::io::{self, BufRead};
use std::str::FromStr;
use std::fs::File;
use std::path::Path;

// ==========================================
// 1. CONSTANTES MATEMÁTICAS
// ==========================================
const INICIO_MANHA: u32 = 9 * 60 + 40;
const FIM_MANHA: u32 = 10 * 60;
const INICIO_TARDE: u32 = 16 * 60 + 10;
const FIM_TARDE: u32 = 16 * 60 + 30;

// ==========================================
// 2. GESTOR DE LISTAS OFFLINE (NEUTRO)
// ==========================================
fn carregar_lista(caminho: &str) -> Vec<String> {
    if !Path::new(caminho).exists() {
        let _ = std::fs::write(caminho, ""); // Cria arquivo vazio
    }
    let file = File::open(caminho).unwrap();
    let reader = io::BufReader::new(file);
    reader.lines()
        .filter_map(|linha| linha.ok())
        .filter(|linha| !linha.trim().is_empty())
        .collect()
}

fn salvar_lista(caminho: &str, lista: &[String]) {
    let _ = std::fs::write(caminho, lista.join("\n"));
}

fn escolher_item_dinamico(titulo: &str, arquivo: &str) -> String {
    loop {
        let mut itens = carregar_lista(arquivo);
        println!("\n{}", titulo);
        
        for (i, item) in itens.iter().enumerate() {
            println!("  {} - {}", i + 1, item);
        }
        
        let op_manual = itens.len() + 1;
        let op_add = itens.len() + 2;
        let op_del = itens.len() + 3;

        println!("  {} - ✍️  Digitar manualmente (apenas desta vez)", op_manual);
        println!("  {} - ➕ Salvar novo item na lista", op_add);
        if !itens.is_empty() {
            println!("  {} - 🗑️  Remover item da lista", op_del);
        }

        let mut s = String::new();
        io::stdin().read_line(&mut s).expect("Erro ao ler");
        
        if let Ok(escolha) = s.trim().parse::<usize>() {
            if escolha > 0 && escolha <= itens.len() {
                return itens[escolha - 1].clone();
            } else if escolha == op_manual {
                return ler_texto("✏️  Digite o nome/descrição:");
            } else if escolha == op_add {
                let novo = ler_texto("✏️  O que deseja salvar na lista?");
                itens.push(novo);
                salvar_lista(arquivo, &itens);
                println!("✅ Salvo!");
            } else if escolha == op_del && !itens.is_empty() {
                let d = ler_dado::<usize>("🗑️  Número do item para remover:");
                if d > 0 && d <= itens.len() {
                    itens.remove(d - 1);
                    salvar_lista(arquivo, &itens);
                    println!("🗑️  Removido!");
                }
            }
        }
    }
}

// ==========================================
// 3. AUXILIARES
// ==========================================
fn ler_texto(msg: &str) -> String {
    println!("{}", msg);
    let mut s = String::new();
    io::stdin().read_line(&mut s).unwrap();
    s.trim().to_string()
}

fn ler_dado<T: FromStr>(msg: &str) -> T {
    loop {
        println!("{}", msg);
        let mut s = String::new();
        io::stdin().read_line(&mut s).unwrap();
        if let Ok(v) = s.trim().parse::<T>() { return v; }
        println!("❌ Entrada inválida.");
    }
}

// ==========================================
// 4. CLI
// ==========================================
#[derive(Parser)]
#[command(author, version, about = "Gestor Monitoria", long_about = None)]
struct Cli {
    #[command(subcommand)]
    comando: Comandos,
}

#[derive(Subcommand)]
enum Comandos {
    Interativo,
    Listar,
    Exportar,
    Deletar { #[arg(short, long)] id: i32 },
}

// ==========================================
// 5. MOTOR
// ==========================================
fn descobrir_dia_da_semana(data: &str) -> Result<String, String> {
    let d = NaiveDate::parse_from_str(data, "%d/%m/%Y")
        .map_err(|_| "Formato DD/MM/AAAA obrigatório".to_string())?;
    Ok(match d.weekday() {
        chrono::Weekday::Mon => "Segunda-feira",
        chrono::Weekday::Tue => "Terça-feira",
        chrono::Weekday::Wed => "Quarta-feira",
        chrono::Weekday::Thu => "Quinta-feira",
        chrono::Weekday::Fri => "Sexta-feira",
        chrono::Weekday::Sat => "Sábado",
        chrono::Weekday::Sun => "Domingo",
    }.to_string())
}

fn calcular_tempo_util(inicio_str: &str, total_min: u32) -> Result<u32, String> {
    let partes: Vec<&str> = inicio_str.split(':').collect();
    if partes.len() != 2 { return Err("Hora deve ser HH:MM".to_string()); }
    let h: u32 = partes[0].parse().unwrap_or(0);
    let m: u32 = partes[1].parse().unwrap_or(0);
    
    let inicio = h * 60 + m;
    let fim = inicio + total_min;
    let mut desc = 0;

    if inicio < FIM_MANHA && fim > INICIO_MANHA {
        desc += fim.min(FIM_MANHA) - inicio.max(INICIO_MANHA);
    }
    if inicio < FIM_TARDE && fim > INICIO_TARDE {
        desc += fim.min(FIM_TARDE) - inicio.max(INICIO_TARDE);
    }

    let tempo = total_min - desc;
    if tempo == 0 || tempo % 50 != 0 {
        return Err(format!("Tempo útil ({}min) deve ser múltiplo de 50.", tempo));
    }
    Ok(tempo)
}

// ==========================================
// 6. DB
// ==========================================
fn abrir_conexao() -> Connection {
    let conn = Connection::open("meus_registros.db").unwrap();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS atividades (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            data TEXT, dia TEXT, horario TEXT, prof TEXT, min INTEGER, desc TEXT
        )", (),
    ).unwrap();
    conn
}

// ==========================================
// 7. EXCEL
// ==========================================
fn gerar_excel(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare("SELECT data, dia, horario, prof, min, desc FROM atividades")?;
    let mut registros: Vec<_> = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, 
            row.get::<_, String>(3)?, row.get::<_, u32>(4)?, row.get::<_, String>(5)?))
    })?.filter_map(|r| r.ok()).collect();

    if registros.is_empty() { return Err("Nada para exportar.".into()); }

    registros.sort_by_key(|r| NaiveDate::parse_from_str(&r.0, "%d/%m/%Y").unwrap());

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    let bold = Format::new().set_bold().set_background_color(Color::Silver);

    let heads = ["Data", "Dia", "Hora", "Prof", "Min", "Desc", "Semana", "Total Semana", "", "Total Geral"];
    for (i, h) in heads.iter().enumerate() {
        let _ = worksheet.write_string_with_format(0, i as u16, *h, &bold);
    }

    let mut l = 1;
    let (mut s_iso, mut s_cnt) = (0, 0);

    for r in registros {
        let dt = NaiveDate::parse_from_str(&r.0, "%d/%m/%Y").unwrap();
        let week = dt.iso_week().week();
        if s_iso == 0 || week != s_iso { s_iso = week; s_cnt += 1; }

        let _ = worksheet.write_string(l, 0, &r.0);
        let _ = worksheet.write_string(l, 1, &r.1);
        let _ = worksheet.write_string(l, 2, &r.2);
        let _ = worksheet.write_string(l, 3, &r.3);
        let _ = worksheet.write_number(l, 4, r.4);
        let _ = worksheet.write_string(l, 5, &r.5);
        let _ = worksheet.write_string(l, 6, format!("Semana {}", s_cnt));
        let _ = worksheet.write_formula(l, 7, format!("=SUMIF(G:G, G{}, E:E)", l + 1).as_str());
        l += 1;
    }
    let _ = worksheet.write_formula(1, 9, format!("=SUM(E2:E{})", l).as_str());
    workbook.save("Relatorio_Monitoria.xlsx")?;
    Ok(())
}

// ==========================================
// 8. MAIN
// ==========================================
fn main() {
    let cli = Cli::parse();
    let conn = abrir_conexao();

    match cli.comando {
        Comandos::Interativo => {
            let data = ler_texto("📅 Data (DD/MM/AAAA):");
            let hora = ler_texto("⏰ Início (HH:MM):");
            let prof = escolher_item_dinamico("👨‍🏫 Professor:", "professores.txt");
            let mins = ler_dado::<u32>("⏳ Duração Total (min):");
            let desc = escolher_item_dinamico("📝 Descrição:", "descricoes.txt");

            let dia = match descobrir_dia_da_semana(&data) { Ok(d) => d, Err(e) => { println!("❌ {}", e); return; }};
            let util = match calcular_tempo_util(&hora, mins) { Ok(u) => u, Err(e) => { println!("❌ {}", e); return; }};

            let _ = conn.execute(
                "INSERT INTO atividades (data, dia, horario, prof, min, desc) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![data, dia, hora, prof, util, desc],
            );
            println!("✅ Salvo com {}min úteis!", util);
        }
        Comandos::Listar => {
            let mut stmt = conn.prepare("SELECT id, data, horario, prof, min FROM atividades").unwrap();
            let rows = stmt.query_map([], |r| Ok((r.get::<_, i32>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, i32>(4)?))).unwrap();
            println!("\nID | DATA | HORA | PROF | MIN");
            for r in rows {
                let (id, dt, hr, pr, mi) = r.unwrap();
                println!("{} | {} | {} | {} | {}", id, dt, hr, pr, mi);
            }
        }
        Comandos::Exportar => {
            if let Err(e) = gerar_excel(&conn) { println!("❌ {}", e); } else { println!("✅ Excel Gerado!"); }
        }
        Comandos::Deletar { id } => {
            let _ = conn.execute("DELETE FROM atividades WHERE id = ?1", params![id]);
            println!("🗑️  Removido!");
        }
    }
}