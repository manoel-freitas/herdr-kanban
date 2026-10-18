# Guia do usuário

Como usar o quadro Kanban do herdr-kanban no dia a dia.

## 1. Primeiro uso

1. Instale o plugin (ver [README](../README.md#instalação)) e confirme que o `gh` está autenticado: `gh auth status`.
2. No herdr, deixe focado um workspace que seja um repositório git com remote no GitHub.
3. Abra o quadro:
   - pelo atalho `prefix+i` (`ctrl+b`, depois `i`; ver README), ou
   - pela paleta de ações do herdr: **Open Kanban board**, ou
   - pela CLI: `herdr plugin action invoke open --plugin manoel-freitas.kanban`.

O quadro abre como overlay sobre o painel ativo. O repositório vem do workspace focado (`gh repo view`). Enquanto carrega, o rodapé mostra `loading…`.

> O overlay abre sobre o **painel ativo**; por isso o workspace certo precisa estar focado antes de abrir.

## 2. Lendo o quadro

```
┌Todo (3)────────┐┌In Progress (1)─┐┌Review (0)──────┐┌Done (20)───────┐┌Agents (no issue) (1)┐
│#41 Auditoria…  ││#33 Check-in…   ││                ││#76 Concluir…   ││🤖 omp w6:p3 idle     │
│unassigned      ││🤖 omp w6:p1 wor││                ││unassigned      ││π rsvp-system         │
└────────────────┘└────────────────┘└────────────────┘└────────────────┘└─────────────────────┘
←→/hl col  ↑↓/jk card  H/L move  ⏎ focus agent  o open  r refresh  q quit | owner/repo · refreshed 5s ago
```

### Colunas

| Coluna | Regra |
|--------|-------|
| Todo | issue aberta sem label de status |
| In Progress | issue aberta com `status:in-progress` |
| Review | issue aberta com `status:review` (vale mais que `status:in-progress`) |
| Done | issue fechada (mostra só as 20 mais recentes) |
| Agents (no issue) | agentes herdr do repositório que não casam com nenhuma issue. Só aparece quando há algum |

O título de cada coluna mostra a contagem. O quadro carrega até 200 issues abertas.

### Cards

Cada card tem duas linhas:

1. `#número título`
2. quem está trabalhando, por ordem de prioridade:
   - agentes herdr ligados à issue: `🤖 <agente> <pane> <status>`
   - assignees do GitHub: `@login`
   - label `agent:<nome>`: `🤖 <nome> (offline)`
   - nenhum dos anteriores: `unassigned`

A cor da segunda linha segue o status do primeiro agente ligado:

| Status | Cor |
|--------|-----|
| working | amarelo |
| blocked | vermelho |
| done | verde |
| idle / unknown | cinza |

### Como um agente é ligado a uma issue

Um agente aparece no card da issue `N` quando:

- o diretório (`cwd`) do agente termina em `issue-N`, por exemplo `.worktrees/issue-33`; ou
- o título do terminal do agente contém `#N`.

Um agente pode aparecer em várias issues. Só entram agentes dos workspaces do mesmo repositório (worktrees do mesmo repo contam). Convenção recomendada: crie a worktree de cada issue como `<repo>/.worktrees/issue-<N>`.

## 3. Navegando

| Tecla | Ação |
|-------|------|
| `h` / `←` | coluna à esquerda |
| `l` / `→` | coluna à direita |
| `j` / `↓` | card abaixo |
| `k` / `↑` | card acima |

## 4. Movendo cards

Selecione um card e use:

- `L`: move para a coluna seguinte (Todo → In Progress → Review → Done)
- `H`: move para a coluna anterior

O card muda de coluna na hora e a seleção o acompanha. Em seguida o plugin grava no GitHub e recarrega para confirmar:

| Destino | O que acontece no GitHub |
|---------|--------------------------|
| Todo / In Progress / Review | adiciona o label `status:*` do destino e remove os outros `status:*`. Se a issue estava fechada, reabre. O label é criado com cor padrão se não existir |
| Done | remove os labels `status:*` e fecha a issue |

Notas:

- Mover para fora das pontas (`H` em Todo, `L` em Done) não faz nada.
- Não dá para mover a partir da coluna `Agents (no issue)`.
- Se a gravação falhar (sem permissão, rede), o erro aparece no rodapé e o recarregamento seguinte devolve o card ao lugar real.

## 5. Ações no card

| Tecla | Ação |
|-------|------|
| `Enter` | foca o painel do primeiro agente ligado ao card e fecha o quadro. Em itens da coluna de agentes, foca aquele agente. Sem agente: `no agent linked` no rodapé |
| `o` | abre a issue no navegador (`xdg-open` no Linux, `open` no macOS) |
| `r` | recarrega issues e agentes agora |
| `q` / `Esc` | fecha o quadro |

### Mouse

| Gesto | Ação |
|-------|------|
| passar o mouse | destaca o card sob o cursor (fundo cinza; o card selecionado mantém o destaque de seleção) |
| clique | seleciona o card (ou a coluna, se clicar em área vazia) |
| duplo clique | foca o agente do card e fecha o quadro (como `Enter`) |
| arrastar e soltar em outra coluna | move o card, inclusive saltando colunas (ex.: Todo → Done). Escreve no GitHub como `H` / `L` |
| roda do mouse | seleciona o card anterior / seguinte na coluna sob o cursor |

Soltar o card na mesma coluna não faz nada. Se o mouse não responder dentro do herdr, confira se `ui.mouse_capture` está habilitado na config do herdr.

## 6. Atualização automática

- Agentes: a cada 3 s.
- Issues: a cada 30 s (e depois de cada movimento).

O rodapé mostra `<owner/repo> · refreshed Ns ago` quando está tudo bem, ou o último erro.

## 7. Dicas de fluxo

- **Acompanhar agentes:** crie a worktree da issue como `.worktrees/issue-N`, rode o agente lá e o card passa a mostrar o status dele ao vivo.
- **Pular para o agente:** navegue até o card e aperte `Enter`.
- **Repo diferente do workspace:** `HERDR_KANBAN_REPO=owner/nome` força o repositório, ignorando a detecção via `gh`. Útil para rodar fora do herdr: `HERDR_KANBAN_REPO=owner/nome herdr-kanban`.
- **Agente humano/offline:** use o label `agent:<nome>` na issue para registrar quem é o dono quando não há agente rodando.

## 8. Problemas comuns

| Mensagem | Causa e solução |
|----------|-----------------|
| `workspace has no git worktree` | o workspace focado não é um repositório git. Foque outro workspace e reabra |
| `no GitHub repo for <dir>: … Set HERDR_KANBAN_REPO=owner/name` | o diretório não tem remote do GitHub. Defina `HERDR_KANBAN_REPO` |
| `<dir>: not a git repo` | rodou fora de um repositório git (modo avulso) |
| `gh issue list …: <erro>` | repo inexistente, sem acesso ou `gh` sem login. Rode `gh auth status` |
| `herdr agent list …` no rodapé | herdr não está rodando; o quadro continua mostrando as issues, sem agentes |
| Overlay não abre no workspace desejado | o overlay abre sobre o painel ativo: foque o workspace antes |
| Card volta para a coluna anterior | a gravação no GitHub falhou; veja o erro no rodapé |

Logs do plugin: `herdr plugin log list --plugin manoel-freitas.kanban`.
