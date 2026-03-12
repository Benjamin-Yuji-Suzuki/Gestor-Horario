# ⏱️ Gestor de Horas Acadêmico (CLI)

Um sistema de linha de comando (CLI) 100% funcional construído em Rust para gerenciar, validar e exportar horas de atividades e monitoria. Este software foca em **privacidade total** e **gestão dinâmica offline**.

> [!IMPORTANT]
> **Aviso de Autoria:** Este projeto foi idealizado, estruturado e validado por **Benjamin (@the0hax)**. 
> O desenvolvimento do código e a documentação contaram com o auxílio de Inteligência Artificial (Gemini/Google) para otimização de sintaxe e implementação de padrões de engenharia em Rust. A lógica de negócio, regras de privacidade e arquitetura funcional são de autoria do usuário.

## 🚀 Funcionalidades
* **Privacidade Local:** Nomes de professores e motivos são salvos em arquivos `.txt` locais, ignorados pelo Git, garantindo que dados pessoais nunca subam para a nuvem.
* **Gestão Dinâmica:** Interface interativa para adicionar, remover ou selecionar opções de listas diretamente no terminal.
* **Validação Matemática:** Desconto automático de intervalos (09:40-10:00 e 16:10-16:30) e exigência de múltiplos de 50 minutos.
* **Exportação Profissional:** Gera planilha Excel (`Relatorio_Monitoria.xlsx`) com agrupamento semanal e fórmulas automáticas.

## 🛠️ Tecnologias Utilizadas
* **Rust** (Linguagem base)
* **Clap** (Interface CLI)
* **Rusqlite** (SQLite embarcado)
* **Chrono** (Cálculo de datas e semanas ISO)
* **Rust_xlsxwriter** (Geração de relatórios Excel)

## 💻 Ambiente de Desenvolvimento
* **SO:** Pop!_OS (Linux)
* **Hardware:** Acer Nitro V15 (i5-13420H, 16GB RAM, RTX 4060)

## ⚙️ Instalação
1. Clone o repositório:
```bash
git clone [https://github.com/SEU_USUARIO/monitoria.git](https://github.com/SEU_USUARIO/monitoria.git)
cd monitoria
```

2. Instale globalmente no sistema:
```bash
cargo install --path .
```

## 📖 Como Usar
Após a instalação, use o comando `monitoria` (ou o nome definido no seu Cargo.toml):

**Assistente Interativo (Recomendado):**
```bash
monitoria interativo
```
*No primeiro uso, o programa criará os arquivos de lista. Use a opção "Adicionar nova opção" para cadastrar seus professores.*

**Outros Comandos:**
* `monitoria listar`: Visualiza os registros no terminal.
* `monitoria exportar`: Gera o relatório em Excel.
* `monitoria deletar -i <ID>`: Remove um registro específico.

> **Nota:** No LibreOffice, utilize `Ctrl + Shift + F9` para recalcular as fórmulas da planilha gerada.
