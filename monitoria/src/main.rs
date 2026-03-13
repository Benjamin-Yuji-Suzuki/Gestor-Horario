use clap::{Parser, Subcommand};
use rusqlite::{params, Connection};
use chrono::{Datelike, Local, NaiveDate};
use rust_xlsxwriter::{Workbook, Format, Color};
use std::io::{self, Write, BufRead};
use std::fs::{self, File};
use std::path::Path;
use std::process::Command;
use directories::UserDirs;

#[derive(Parser)]
#[command(author, version, about = "Monitoria Pro")]
struct Cli { #[command(subcommand)] comando: Comandos }

#[derive(Subcommand)]
enum Comandos {
    #[command(visible_alias = "reg")] Interativo,
    #[command(visible_aliases = ["l", "lis"])] Listar,
    #[command(visible_alias = "exp")] Exportar,
    #[command(visible_alias = "att")] Atualizar { id: i32 },
    #[command(visible_alias = "del")] Deletar { id: i32 },
    #[command(visible_alias = "int")] Intervalos,
}

struct Intervalo { inicio: u32, fim: u32 }

// ==========================================
// GESTOR DE INTERVALOS
// ==========================================
fn carregar_intervalos(proativo: bool) -> Vec<Intervalo> {
    let caminho = "intervalos.txt";
    if !Path::new(caminho).exists() {
        let _ = fs::write(caminho, "");
        if proativo {
            println!("\n🆕 Arquivo de intervalos criado!");
            let resp = ler("Deseja configurar seus horários de descanso agora? (s/N): ").to_lowercase();
            if resp == "s" {
                let _ = Command::new("nano").arg(caminho).status();
                println!("✅ Intervalos salvos!");
            }
        }
    }

    let file = File::open(caminho).unwrap();
    let mut lista = Vec::new();
    for linha in io::BufReader::new(file).lines().filter_map(|l| l.ok()) {
        let partes: Vec<&str> = linha.split('-').collect();
        if partes.len() == 2 {
            let parse_hora = |s: &str| {
                let h_m: Vec<u32> = s.split(':').filter_map(|v| v.parse().ok()).collect();
                if h_m.len() == 2 { h_m[0] * 60 + h_m[1] } else { 0 }
            };
            lista.push(Intervalo { inicio: parse_hora(partes[0].trim()), fim: parse_hora(partes[1].trim()) });
        }
    }
    lista
}

fn calc_fim(ini_str: &str, dur: u32, intervalos: &[Intervalo]) -> String {
    let p: Vec<u32> = ini_str.split(':').filter_map(|v| v.parse().ok()).collect();
    if p.len() != 2 { return "00:00".into(); }
    let mut tempo = p[0] * 60 + p[1];
    let mut rest = dur;
    while rest > 0 {
        tempo += 1;
        let em_intervalo = intervalos.iter().any(|i| tempo > i.inicio && tempo <= i.fim);
        if !em_intervalo { rest -= 1; }
    }
    format!("{:02}:{:02}", tempo / 60, tempo % 60)
}

// ==========================================
// GESTOR DE LISTAS
// ==========================================
fn carregar_lista(caminho: &str) -> Vec<String> {
    if !Path::new(caminho).exists() { let _ = fs::write(caminho, ""); }
    let file = File::open(caminho).unwrap();
    io::BufReader::new(file).lines().filter_map(|l| l.ok()).filter(|l| !l.trim().is_empty()).collect()
}

fn escolher_item_dinamico(titulo: &str, arquivo: &str, atual: Option<&str>) -> String {
    loop {
        let mut itens = carregar_lista(arquivo);
        println!("\n--- SELEÇÃO DE {} ---", titulo.to_uppercase());
        for (i, item) in itens.iter().enumerate() { println!("  {} - {}", i + 1, item); }
        let op_manual = itens.len() + 1;
        let op_add = itens.len() + 2;
        let op_del = itens.len() + 3;
        println!("  {} - ✍️  Manual | {} - ➕ Salvar Novo | {} - 🗑️  Remover", op_manual, op_add, op_del);
        let p = match atual { Some(v) => format!("Opção (Enter mantém \"{}\"): ", v), None => "Opção: ".to_string() };
        print!("{}", p); io::stdout().flush().unwrap();
        let mut s = String::new(); io::stdin().read_line(&mut s).unwrap();
        let s = s.trim();
        if s.is_empty() { if let Some(v) = atual { return v.to_string(); } continue; }
        if let Ok(e) = s.parse::<usize>() {
            if e > 0 && e <= itens.len() { return itens[e - 1].clone(); }
            else if e == op_manual { return ler("✏️  Valor: "); }
            else if e == op_add {
                let n = ler("✨ Novo: ");
                if !n.is_empty() { itens.push(n.clone()); let _ = fs::write(arquivo, itens.join("\n")); return n; }
            } else if e == op_del && !itens.is_empty() {
                let d = ler("🗑️  Número: ").parse::<usize>().unwrap_or(0);
                if d > 0 && d <= itens.len() { itens.remove(d - 1); let _ = fs::write(arquivo, itens.join("\n")); }
                continue;
            }
        }
    }
}

fn ler(m: &str) -> String { print!("{}", m); io::stdout().flush().unwrap(); let mut s = String::new(); io::stdin().read_line(&mut s).unwrap(); s.trim().to_string() }

fn main() {
    let cli = Cli::parse();
    let conn = Connection::open("meus_registros.db").unwrap();
    let _ = conn.execute("CREATE TABLE IF NOT EXISTS atividades (id INTEGER PRIMARY KEY AUTOINCREMENT, data TEXT, dia TEXT, horario_inicio TEXT, horario_fim TEXT, prof TEXT, min INTEGER, desc TEXT)", []);

    match cli.comando {
        Comandos::Intervalos => {
            let caminho = "intervalos.txt";
            if !Path::new(caminho).exists() { let _ = fs::write(caminho, ""); }
            println!("📂 Abrindo editor de intervalos...");
            let _ = Command::new("nano").arg(caminho).status().expect("Falha ao abrir o editor.");
            println!("✅ Intervalos atualizados!");
        }

        Comandos::Interativo => {
            let intervalos = carregar_intervalos(true); // Pergunta se o arquivo for novo
            let hj = Local::now().format("%d/%m/%Y").to_string();
            let dt = { let i = ler(&format!("📅 Data ({}): ", hj)); if i.is_empty() { hj } else { i } };
            let hr_i = ler("⏰ Início (HH:MM): ");
            let mins: u32 = ler("⏳ Minutos: ").parse().unwrap_or(0);
            let hr_f = calc_fim(&hr_i, mins, &intervalos);
            let prof = escolher_item_dinamico("Professor", "professores.txt", None);
            let desc = escolher_item_dinamico("Descrição", "descricoes.txt", None);
            let _ = conn.execute("INSERT INTO atividades (data, horario_inicio, horario_fim, prof, min, desc) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![dt, hr_i, hr_f, prof, mins, desc]);
            println!("✅ Salvo! Término: {}", hr_f);
        }

        Comandos::Listar => {
            let _ = carregar_intervalos(false); // Só carrega, sem perguntar
            let mut stmt = conn.prepare("SELECT id, data, horario_inicio, horario_fim, prof, min FROM atividades").unwrap();
            let rows = stmt.query_map([], |r| Ok((r.get::<_, i32>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, u32>(5)?))).unwrap();
            println!("\nID | DATA       | INÍCIO | FIM   | PROFESSOR     | MIN\n{}", "-".repeat(65));
            let (mut s, mut t, hj) = (0, 0, Local::now().date_naive());
            for r in rows {
                let (id, dt, hi, hf, pr, mi) = r.unwrap(); t += mi;
                if let Ok(d) = NaiveDate::parse_from_str(&dt, "%d/%m/%Y") { if d.iso_week().week() == hj.iso_week().week() { s += mi; } }
                println!("{:<2} | {:<10} | {:<6} | {:<5} | {:<13} | {}min", id, dt, hi, hf, pr, mi);
            }
            println!("{}\n📅 Semana: {}h {:02}min | 🌎 Total: {}h {:02}min", "-".repeat(65), s/60, s%60, t/60, t%60);
        }

        Comandos::Atualizar { id } => {
            let intervalos = carregar_intervalos(false);
            let res = conn.query_row("SELECT data, horario_inicio, prof, min, desc FROM atividades WHERE id = ?1", [id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, u32>(3)?, r.get::<_, String>(4)?)));
            if let Ok((d, h, p, m, ds)) = res {
                let dt = { let i = ler(&format!("Data ({}): ", d)); if i.is_empty() { d } else { i } };
                let hi = { let i = ler(&format!("Início ({}): ", h)); if i.is_empty() { h } else { i } };
                let pr = escolher_item_dinamico("Professor", "professores.txt", Some(&p));
                let mins = { let i = ler(&format!("Minutos ({}): ", m)); if i.is_empty() { m } else { i.parse().unwrap_or(m) } };
                let desc = escolher_item_dinamico("Descrição", "descricoes.txt", Some(&ds));
                let hf = calc_fim(&hi, mins, &intervalos);
                let _ = conn.execute("UPDATE atividades SET data=?1, horario_inicio=?2, horario_fim=?3, prof=?4, min=?5, desc=?6 WHERE id=?7", params![dt, hi, hf, pr, mins, desc, id]);
                println!("✅ Registro atualizado!");
            }
        }

        Comandos::Exportar => {
            let mut stmt = conn.prepare("SELECT data, horario_inicio, horario_fim, prof, min, desc FROM atividades").unwrap();
            let regs: Vec<_> = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, u32>(4)?, r.get::<_, String>(5)?))).unwrap().filter_map(|r| r.ok()).collect();
            let mut wb = Workbook::new(); let ws = wb.add_worksheet();
            let bold = Format::new().set_bold().set_background_color(Color::Silver);
            let h = ["Data", "Início", "Fim", "Professor", "Minutos", "Descrição"];
            for (i, v) in h.iter().enumerate() { let _ = ws.write_string_with_format(0, i as u16, *v, &bold); }
            let mut l = 1;
            for r in regs {
                let _ = ws.write_string(l, 0, &r.0); let _ = ws.write_string(l, 1, &r.1);
                let _ = ws.write_string(l, 2, &r.2); let _ = ws.write_string(l, 3, &r.3);
                let _ = ws.write_number(l, 4, r.4 as f64); let _ = ws.write_string(l, 5, &r.5);
                l += 1;
            }
            let path = UserDirs::new().and_then(|ud| ud.desktop_dir().map(|d| d.join("Relatorio_Monitoria.xlsx"))).map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| "Relatorio_Monitoria.xlsx".to_string());
            let _ = wb.save(path); println!("✅ Excel Exportado!");
        }

        Comandos::Deletar { id } => { let _ = conn.execute("DELETE FROM atividades WHERE id = ?1", params![id]); println!("🗑️  Removido!"); }
    }
}