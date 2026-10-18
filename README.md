# herdr-kanban

Plugin do [herdr](https://github.com/herdrdev/herdr) que abre um quadro Kanban das issues do GitHub do workspace atual. Cada card mostra quem está trabalhando nele: um agente herdr ao vivo (com status) ou o assignee humano.

## Requisitos

- `gh` autenticado (`gh auth status`)
- Toolchain Rust (`cargo`)
- herdr >= 0.9.0

## Instalação

```sh
herdr plugin install manoel-freitas/herdr-kanban
```

Desenvolvimento local:

```sh
cargo build --release && herdr plugin link .
```

Atalho (em `config.toml` do herdr):

```toml
[[keys.command]]
key = "prefix+k"
type = "plugin_action"
command = "manoel-freitas.kanban.open"
description = "kanban"
```

## Convenções

- Colunas por label: `status:todo`, `status:in-progress`, `status:review`. Issue fechada = Done. Sem label de status = Todo. Com `status:review` e `status:in-progress` ao mesmo tempo, vale Review.
- Um agente é ligado a uma issue quando o `cwd` termina em `issue-<N>` (ex.: `.worktrees/issue-33`) ou quando o título do terminal contém `#N`. Agentes sem issue aparecem na coluna `Agents (no issue)`.
- Label `agent:<nome>` marca um agente dono offline.
- `HERDR_KANBAN_REPO=owner/nome` força o repositório, ignorando a detecção via `gh`.

## Teclas

| Tecla | Ação |
|-------|------|
| `h` / `←`, `l` / `→` | trocar de coluna |
| `j` / `↓`, `k` / `↑` | trocar de card |
| `H` / `L` | mover card para a coluna anterior / seguinte (escreve no GitHub) |
| `Enter` | focar o painel do agente ligado ao card e fechar o quadro |
| `o` | abrir a issue no navegador |
| `r` | atualizar |
| `q` / `Esc` | sair |

Issues atualizam a cada 30 s, agentes a cada 3 s.

## Licença

GPLv3 (ver `LICENSE`).
