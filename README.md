# herdr-kanban

Plugin do [herdr](https://github.com/herdrdev/herdr) que abre um quadro Kanban das issues do GitHub do workspace atual. Cada card mostra quem está trabalhando nele: um agente herdr ao vivo (com status) ou o assignee humano. Dá para mover cards entre colunas (grava no GitHub), pular para o painel do agente e abrir a issue no navegador.

Guia completo: [docs/USER_GUIDE.md](docs/USER_GUIDE.md).

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

Atalho: o `plugin install` adiciona `prefix+i` ao seu `config.toml` do herdr (idempotente; o prefix padrão é `ctrl+b`, então `ctrl+b` e depois `i`). Depois rode `herdr server reload-config`. Com `plugin link` o script não roda; adicione à mão:

```toml
[[keys.command]]
key = "prefix+i"
type = "plugin_action"
command = "manoel-freitas.kanban.open"
description = "kanban"
```

`prefix+k` não serve: o herdr já usa para `focus_pane_up`. Para trocar a tecla, edite o `key` no seu `config.toml`.

## Como usar

1. Foque um workspace do herdr que seja um repositório git com remote no GitHub.
2. Abra o quadro: `prefix+i` (`ctrl+b`, `i`), a ação **Open Kanban board** ou `herdr plugin action invoke open --plugin manoel-freitas.kanban`. Ele abre como overlay sobre o painel ativo.
3. Navegue com `h j k l` (ou setas), mova o card selecionado com `H` / `L`, aperte `Enter` para focar o agente do card e `q` para sair.

Colunas: **Todo**, **In Progress**, **Review**, **Done**, mais **Agents (no issue)** quando há agentes sem issue. Cada card mostra na 2ª linha o agente (`🤖 omp w6:p1 working`), o assignee (`@login`) ou `unassigned`.

Para um agente aparecer no card da issue `N`, rode-o numa worktree `.worktrees/issue-N` ou ponha `#N` no título do terminal.

Detalhes (regras de colunas, movimentos, cores, atualização automática, problemas comuns) no [guia do usuário](docs/USER_GUIDE.md).

## Convenções

- Colunas por label: `status:todo`, `status:in-progress`, `status:review`. Issue fechada = Done. Sem label de status = Todo. Com `status:review` e `status:in-progress` ao mesmo tempo, vale Review.
- Um agente é ligado a uma issue quando o `cwd` termina em `issue-<N>` (ex.: `.worktrees/issue-33`) ou quando o título do terminal contém `#N`.
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
