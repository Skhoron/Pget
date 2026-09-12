# pget

Небольшая консольная утилита для генерации криптографически стойких
случайных значений заданной длины в битах. Часть проекта
[Skhoron](https://github.com/Skhoron).

## Что делает pget?

Логика простая:

1. Пользователь указывает количество бит.
2. pget запрашивает у операционной системы случайные байты.
3. Если запрошенный размер не кратен 8, pget маскирует лишние старшие биты.
4. pget выводит полученное значение.

```text
pget generate 8
pget generate 128
pget generate 256
pget generate 1024
pget generate 4096
```

Можно запросить любое положительное количество бит.

## Пример

```text
$ pget generate 256
Bits: 256
Bytes: 32
Format: hex

9f31c7a4d8...c92a
```

Для размера, не кратного байту:

```text
$ pget generate 13
Bits: 13
Bytes: 2
Format: hex

1a1f
```

Значимы только запрошенные 13 бит — лишние старшие биты последнего байта
обнуляются перед кодированием.

## Команды

```text
pget generate <bits> [--format hex|binary|decimal|base64]
pget info <bits>
pget --help
pget --version
```

`generate` выводит случайное значение. `info` показывает метаданные о
размере, ничего не генерируя:

```text
$ pget info 256
Bits:             256
Bytes:            32
Hex characters:   64
Possible values:  2^256
```

## Форматы вывода

По умолчанию `generate` выводит hex. Другие форматы задаются через
`--format`:

```text
pget generate 256 --format hex        # по умолчанию
pget generate 256 --format binary
pget generate 256 --format decimal
pget generate 256 --format base64
```

`--format decimal` конвертирует буфер в десятичную строку через
деление всего числа на 10^9 за проход — по своей природе это
квадратичная по длине операция (в отличие от hex/binary/base64,
которые линейны). Поэтому `decimal` ограничен отдельным, гораздо более
строгим потолком — 131072 бит (16 КиБ), а не общим `MAX_BITS`. Для
больших значений используйте `hex`, `binary` или `base64`.

## Библиотека

CLI — это тонкая обвязка над `src/lib.rs`, который можно использовать
напрямую из другого Rust-кода:

```rust
let bits = 256;
let value = pget::generate_hex(bits)?;       // случайное значение в hex
let raw = pget::generate_bytes(bits)?;       // случайное значение как Vec<u8>
let bytes_needed = pget::byte_length(13);    // 2
```

Публичные функции: `generate_bytes`, `generate_hex`, `generate_binary`,
`generate_decimal`, `generate_base64`, `validate_bits`, `byte_length`,
`mask_unused_bits`, форматтеры `format_*`, а также константа `MAX_BITS`.
Ядро — `generate_bytes`: она валидирует длину, берёт байты из
CSPRNG операционной системы и маскирует лишние биты. Остальные
`generate_*` вызывают её и форматируют результат — отдельного алиаса
`generate()` нет, поскольку это было бы просто второе имя для того же
самого.

## Структура проекта

```text
pget/
├── .github/
│   └── workflows/
│       └── ci.yml
├── src/
│   ├── lib.rs
│   └── main.rs
├── tests/
│   └── cli.rs
├── Cargo.toml
├── Cargo.lock
├── LICENSE
├── README.md
└── .gitignore
```

## Модель безопасности

pget использует криптографически стойкий источник случайности
операционной системы (через `rand::rngs::OsRng`).

Он **не** использует:

- метки времени;
- идентификаторы процессов;
- предсказуемые счётчики;
- некриптографические реализации `rand()`;
- вручную сконструированные псевдослучайные seed-значения.

pget не сохраняет и не передаёт сгенерированные значения — всё
происходит локально, значение печатается один раз в stdout.

## Длина в битах

pget принимает любое положительное количество бит вплоть до
`pget::MAX_BITS` (2^30, то есть 128 МиБ вывода) — этого достаточно для
любого реального ключа или nonce, но граница не даёт опечатке в
значении заставить pget попытаться выделить гигабайты памяти. Для N
бит число возможных значений равно 2^N. Например:

| Биты | Число возможных значений |
|------|---------------------------|
| 8    | 2^8                       |
| 128  | 2^128                     |
| 256  | 2^256                     |
| 1024 | 2^1024                    |
| 4096 | 2^4096                    |

### Значения, не кратные байту

Байт — это 8 бит. Если запрошенный размер не делится на 8, pget
выделяет достаточное число байт и обнуляет лишние старшие биты
последнего байта. Например, 13 бит требуют 2 байта: первый байт
значим полностью (8 бит), второй байт даёт свои младшие 5 бит, а
верхние 3 бита обнуляются. Это гарантирует, что итоговое значение
имеет ровно запрошенную длину в битах.

## Обработка ошибок

Некорректный ввод отклоняется:

```text
$ pget generate 0
Error: bit length must be greater than zero

$ pget generate -256
Error: bit length must be positive

$ pget generate abc
Error: invalid bit length

$ pget generate 99999999999
Error: bit length must not exceed 1073741824 (got 99999999999)

$ pget generate 9999999 --format decimal
Error: --format decimal only supports up to 131072 bits (got 9999999); use hex, binary, or base64 for larger values
```

## Сборка

Нужны Rust и Cargo.

```sh
cargo build --release
```

Собранный бинарник будет доступен в `target/release/`.

## Тесты

Запустить весь набор тестов (юнит-тесты + CLI-интеграционные):

```sh
cargo test
```

Проверить форматирование:

```sh
cargo fmt --check
```

Запустить Clippy:

```sh
cargo clippy --all-targets --all-features -- -D warnings
```

## Continuous Integration

GitHub Actions на каждый push и pull request запускает
`cargo fmt --check`, `cargo check`, `cargo test` и `cargo clippy`.
Workflow лежит в `.github/workflows/ci.yml`.

## Лицензия

MIT License. Copyright (c) 2026 Skhoron.