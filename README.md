# Oxide Calc

Calculadora desktop para Linux em Rust + Slint, seguindo o Material Design 3. É o primeiro app do ecossistema **Oxide** e usa o design system [oxide-ui](https://github.com/william-franco/oxide-ui).

## Funcionalidades

- Modo básico: `+ − × ÷`, porcentagem (`200 + 10%` = 220), inverter sinal, apagar e limpar.
- Modo científico: `sin cos tan` e inversas, `ln log`, `√ ∛`, `x² xʸ 10ˣ 1/x`, `x!`, `π e`, parênteses e graus/radianos.
- Prévia do resultado durante a digitação. Os parênteses abertos são fechados automaticamente.
- Histórico de até 100 itens: clique num item para reutilizar a expressão. O histórico pode ser limpo.
- Tema claro e escuro. O tema inicial segue o sistema.
- Layout responsivo:
  - Abaixo de 600 px: o histórico abre em um painel inferior recolhível.
  - De 600 a 839 px: as funções científicas ficam ao lado do teclado.
  - A partir de 840 px: o histórico fica num painel lateral fixo.

## Teclado

| Tecla | Ação |
|---|---|
| `0-9` `.` `,` `+` `-` `*` `/` `^` `%` `!` `(` `)` | entrada |
| `Enter` / `=` | calcular |
| `Backspace` | apagar |
| `Esc` / `Delete` | limpar (`Esc` fecha o histórico se ele estiver aberto) |
| `s` `c` `t` `l` `g` `r` `p` `e` | sin, cos, tan, ln, log, √, π, e |
| `h` | abrir/fechar histórico |
| `Tab` / `Espaço` | navegar entre os botões / acionar o botão focado |

## Arquitetura

```
src/engine/   motor puro (lexer, parser recursivo, avaliador, formatação, erros) + testes
src/viewmodel.rs   estado da calculadora (expressão, prévia, erro, ângulo, histórico)
src/bridge.rs      liga os callbacks Slint ao ViewModel
ui/                telas Slint (app, teclados, histórico) usando @oxide/oxide.slint
```

Dependências:

- `slint`: UI.
- `slint-build`: compila os arquivos `.slint`.
- `oxide-ui`: design system, consumido via git.

O motor usa só `std`.

## Compilar e executar (Linux)

```
sudo apt install libfontconfig1-dev libxkbcommon-dev libwayland-dev
cargo run --release
cargo test
cargo update -p oxide-ui   # pega a versão mais nova do oxide-ui no GitHub
```

Para desenvolver o oxide-ui localmente ao mesmo tempo, sem alterar o `Cargo.toml`:

```
cargo run --config 'patch."https://github.com/william-franco/oxide-ui".oxide-ui.path="../oxide-ui"'
```

## ScreenShots

| Image 1 | Image 2 | Image 3 |
|----------|----------|----------|
| ![App Screenshot](assets/screenshots/screen-1.png) | ![App Screenshot](assets/screenshots/screen-2.png) | ![App Screenshot](assets/screenshots/screen-3.png) |

## Examples of commits

```
git add . && git commit -m ":rocket: Initial commit." && git push
git add . && git commit -m ":building_construction: Added initial project architecture." && git push
git add . && git commit -m ":building_construction: Update project architecture." && git push
git add . && git commit -m ":memo: Updated project documentation." && git push
git add . && git commit -m ":memo: Updated code documentation." && git push
git add . && git commit -m ":white_check_mark: Added feature xyz." && git push
git add . && git commit -m ":wrench: Fixed xyz usage." && git push
git add . && git commit -m ":heavy_minus_sign: Removed xyz." && git push
git add . && git commit -m ":memo: Adjusted project imports." && git push
git add . && git commit -m ":arrow_up: Updated dependencies." && git push
git add . && git commit -m ":arrow_down: Removed dependencies." && git push
git add . && git commit -m ":wastebasket: Removed unused code." && git push
git add . && git commit -m ":test_tube: Added test functionality xyz." && git push
git add . && git commit -m ":construction_worker: Building in progress." && git push
git add . && git commit -m ":construction_worker: Added CI build system." && git push
```

## License

MIT License

Copyright (c) 2026 William Franco

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.