# Send и Sync в Rust

## Что такое Send и Sync

**`Send`** и **`Sync`** — это **маркерные трейты** (marker traits) из стандартной библиотеки Rust. Они **не имеют методов** — только **помечают** тип как безопасный для **конкурентного** использования.

| Трейт | Что означает |
|---|---|
| **`Send`** | Значение типа можно **переместить** в другой поток |
| **`Sync`** | Ссылку `&T` можно **разделить** между потоками |

## Связь между Send и Sync

**Ключевое правило:**

```
T: Sync  ⇔  &T: Send
```

- Если `&T` можно **переместить** в другой поток → `T: Sync`.
- Если `T: Sync` → `&T: Send`.

**Интуиция:**

- **`Send`** — «можно **отправить** в поток».
- **`Sync`** — «можно **разделить** между потоками».

## Auto traits — компилятор выводит сам

`Send` и `Sync` — **auto traits**. Компилятор **автоматически** выводит реализацию **по полям**:

```rust
struct Config {
    name: String,   // String: Send + Sync
    value: i32,     // i32: Send + Sync
}
// Config: Send + Sync — выведено автоматически
```

**Правило:** если **все поля** `Send` → структура `Send`. Если **все поля** `Sync` → структура `Sync`.

## Разбор примера

```rust
use std::sync::Arc;
use std::thread;

fn main() {
    let v = Arc::new(vec![1, 2, 3]);   // Arc<Vec<i32>>: Send + Sync

    let mut hands = vec![];

    for i in 0..5 {
        hands.push(
            thread::spawn({
                let v_clone = v.clone();   // клон Arc
                move || {
                    println!("Thread: {}, v_clone: {:?}", i, v_clone);
                    println!("*Thread: {}, v_clone: {:?}\n", i, *v_clone);
                }
            })
        );
    }

    for h in hands {
        h.join().unwrap();
    }
}
```

### Что происходит

1. **`Arc::new(vec![1, 2, 3])`** — `Arc<Vec<i32>>`.
   - `Vec<i32>: Send + Sync` → `Arc<Vec<i32>>: Send + Sync`.
2. **`v.clone()`** — клон `Arc`, увеличивает **атомарный** счётчик.
3. **`thread::spawn(move || ...)`** — требует **`Send`** для замыкания.
   - `v_clone: Arc<Vec<i32>>: Send` → ✅.
   - `i: i32: Send` → ✅.
4. **`*v_clone`** — **разыменование** `Arc` → `Vec<i32>`.
5. **5 потоков** читают **одну** `Vec` **одновременно** — благодаря `Sync`.

### Почему `Arc`, а не `Rc`

**`Rc<i32>`** — **`!Send`** и **`!Sync`**:

```rust
use std::rc::Rc;

let v = Rc::new(vec![1, 2, 3]);   // Rc: !Send + !Sync

thread::spawn(move || {           // ❌ ошибка компиляции
    println!("{:?}", v);
});
```

**Ошибка:**

```
error[E0277]: `Rc<Vec<i32>>` cannot be sent between threads safely
```

**Причина:** `Rc` использует **неатомарный** счётчик ссылок. Если два потока одновременно **клонируют** или **уничтожают** `Rc` — **гонка данных** → **UB**.

**`Arc`** использует **атомарный** счётчик → **безопасно**.

## Сырые указатели — `!Send` и `!Sync`

```rust
use std::cell::Cell;
use std::rc::Rc;

type RawPtr = Rc<Cell<*mut i32>>;   // !Send + !Sync
```

**Почему:**

- **`Rc`** — неатомарный счётчик → `!Send`, `!Sync`.
- **`Cell<*mut i32>`** — мутация через `&self` **без синхронизации** → `!Sync`.
- **`*mut i32`** — сырой указатель → `!Send`, `!Sync`.

**Следствие:**

- **`RawPtr` не может** быть в `Arc`.
- **`Arc<Cell<*mut T>>`** — **не** компилируется, потому что `Cell<*mut T>: !Sync`.

## Таблица: какие типы `Send`/`Sync`

| Тип | `Send` | `Sync` | Причина |
|---|---|---|---|
| `i32`, `bool`, `char` | ✅ | ✅ | Примитивы |
| `String`, `Vec<T>` | ✅ | ✅ | Если `T: Send + Sync` |
| `&T` | ✅ | ✅ | Если `T: Sync` |
| `&mut T` | ✅ | ❌ | Если `T: Send` |
| `*const T`, `*mut T` | ❌ | ❌ | Сырые указатели |
| **`Rc<T>`** | ❌ | ❌ | Неатомарный счётчик |
| **`Arc<T>`** | ✅ | ✅ | Если `T: Send + Sync` |
| **`RefCell<T>`** | ✅ | ❌ | Неатомарные заимствования |
| **`Cell<T>`** | ✅ | ❌ | Неатомарная мутация |
| **`Mutex<T>`** | ✅ | ✅ | Если `T: Send` |
| **`RwLock<T>`** | ✅ | ✅ | Если `T: Send + Sync` |
| **`MutexGuard<'_, T>`** | ❌ | ✅ | **Sync, но !Send** |
| **`mpsc::Sender<T>`** | ✅ | ✅ | Если `T: Send` |
| **`mpsc::Receiver<T>`** | ✅ | ❌ | Один получатель |

## Практический пример: `Send` без `Sync`

```rust
use std::cell::RefCell;
use std::thread;

fn main() {
    let data = RefCell::new(42);   // RefCell<i32>: Send + !Sync

    // ✅ Перемещаем в поток (Send)
    thread::spawn(move || {
        *data.borrow_mut() = 100;
        println!("{}", data.borrow());
    }).join().unwrap();

    // ❌ Нельзя разделить &data (не Sync)
    // thread::scope(|s| {
    //     s.spawn(|| { println!("{}", data.borrow()); });
    // });
}
```

**Вывод:**

- `RefCell<i32>: Send` → **можно переместить** в поток.
- `RefCell<i32>: !Sync` → **нельзя разделить** между потоками.

## Практический пример: `Sync` без `Send`

```rust
use std::sync::{Mutex, MutexGuard};

fn main() {
    let m = Mutex::new(42);
    let guard: MutexGuard<'_, i32> = m.lock().unwrap();

    // ✅ Sync: &guard можно разделить
    fn assert_sync<T: Sync>() {}
    assert_sync::<MutexGuard<'_, i32>>();

    // ❌ !Send: нельзя переместить в поток
    // thread::spawn(move || { let _ = guard; });
}
```

**Почему `!Send`:** `MutexGuard` **привязан** к потоку, который захватил мьютекс. Освобождение из **другого** потока → **UB**.

## Сравнение `Arc` и `Rc`

| | `Rc<T>` | `Arc<T>` |
|---|---|---|
| Счётчик | **Неатомарный** | **Атомарный** |
| `Send` | ❌ | ✅ (если `T: Send + Sync`) |
| `Sync` | ❌ | ✅ (если `T: Send + Sync`) |
| Скорость | **Быстрее** | Медленнее (атомики) |
| Многопоточность | ❌ | ✅ |
| Когда использовать | Однопоточный код | Многопоточный код |

## `unsafe impl Send/Sync`

Иногда нужно **вручную** реализовать:

```rust
struct MyType {
    data: *mut i32,   // сырой указатель — !Send, !Sync
}

// Обещаем компилятору, что тип безопасен
unsafe impl Send for MyType {}
unsafe impl Sync for MyType {}
```

**Опасно:** вы **обещаете** компилятору, что тип **действительно** потокобезопасен. Если нет — **UB**.

## Сводная таблица

| Аспект | `Send` | `Sync` |
|---|---|---|
| Что означает | Перемещение между потоками | Разделение `&T` между потоками |
| Auto trait | ✅ Да | ✅ Да |
| Методы | ❌ Нет | ❌ Нет |
| Связь | `T: Sync ⇔ &T: Send` | То же |
| Пример | `i32`, `String`, `Arc<T>` | `i32`, `String`, `Mutex<T>` |
| `!Send` | `Rc<T>`, `*mut T` | — |
| `!Sync` | — | `RefCell<T>`, `Cell<T>` |
| Оба | `i32`, `Arc<T>`, `Mutex<T>` | То же |

## Итог

- **`Send`** — значение можно **переместить** в другой поток.
- **`Sync`** — `&T` можно **разделить** между потоками.
- **Auto traits** — компилятор **выводит** реализацию по полям.
- **`T: Sync ⇔ &T: Send`** — фундаментальная связь.
- **`Rc`** — `!Send`, `!Sync` (неатомарный счётчик).
- **`Arc`** — `Send + Sync` (атомарный счётчик).
- **`RefCell`**, **`Cell`** — `Send`, но `!Sync`.
- **`MutexGuard`** — `Sync`, но `!Send`.
- **Сырые указатели** — `!Send`, `!Sync`.
- **`Arc<Cell<*mut T>>`** — **не** компилируется.
- **`thread::spawn`** требует **`Send`**.
- **`thread::scope`** требует **`Sync`** для разделяемых ссылок.
- **Правило:** `Send` — «отправить», `Sync` — «разделить».
