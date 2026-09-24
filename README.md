# Send и Sync в Rust

# Пояснение кода: `Send`, `Sync`, `Arc`, `RefCell`

## Что демонстрирует код

Пример показывает **три** сценария:

1. **`Arc`** — разделяемое владение `Vec<i32>` между **5 потоками** (работает).
2. **`RefCell`** — **перемещение** в поток через `move` (работает, потому что `RefCell: Send`).
3. **`RefCell`** — **разделение** через `thread::scope` (**не компилируется**, потому что `RefCell: !Sync`).

## Часть 1: `Arc` — разделяемое владение

```rust
let v = Arc::new(vec![1, 2, 3]);

let mut hands = vec![];

for i in 0..5 {
    hands.push(
        thread::spawn({
            let v_clone = v.clone();
            move || {
                println!("Thread: {}, v_clone: {:?}", i, v_clone);
                println!("Thread: {}, *v_clone: {:?}\n", i, *v_clone);
            }
        })
    );
}

for h in hands {
    h.join().unwrap();
}
```

### Что происходит

1. **`Arc::new(vec![1, 2, 3])`** — создаёт `Arc<Vec<i32>>`.
2. **`v.clone()`** — клон `Arc` (**увеличивает** атомарный счётчик).
3. **`thread::spawn(move || ...)`** — перемещает клон в поток.
4. **5 потоков** читают `Vec` **одновременно**.
5. **`h.join()`** — ждём завершения.

### Почему работает

- **`Arc<Vec<i32>>: Send + Sync`**:
  - `Vec<i32>: Send + Sync` → `Arc<Vec<i32>>: Send + Sync`.
- **`v.clone()`** — атомарное увеличение счётчика.
- **`*v_clone`** — `Deref` даёт `&Vec<i32>`, безопасно.

### Порядок вывода **не гарантирован**

Потоки выполняются **параллельно** — порядок строк **случайный**.

## Часть 2: `RefCell` + `move` — перемещение в поток

```rust
let data = RefCell::new(10);

thread::spawn(move || {
    *data.borrow_mut() += 10;
    println!("val: {}", *data.borrow());
})
.join()
.unwrap();
```

### Что происходит

1. **`RefCell::new(10)`** — создаёт `RefCell<i32>`.
2. **`move ||`** — замыкание **перемещает** `data` внутрь.
3. **`*data.borrow_mut() += 10`** — изменяет значение.
4. **`println!`** — читает.

### Почему работает

- **`RefCell<i32>: Send`** — можно **переместить** в поток.
- **`move`** — перемещает **владение** `data` в замыкание.
- **`thread::spawn`** требует **`Send`** → ✅.

### Почему `RefCell: Send`

1. **`i32: Send`** — внутреннее значение можно перемещать.
2. **`RefCell` не содержит `!Send`-полей** (нет `Rc`, сырых указателей).
3. **`Send` — auto trait** — компилятор **выводит** по полям.

## Часть 3: `RefCell` + `thread::scope` — **не компилируется**

```rust
let data = RefCell::new(10);

thread::scope(|s| {
    s.spawn(|| {
        println!("{}", *data.borrow());
    });
});
```

### Что происходит

1. **`thread::scope`** — создаёт **scoped threads**.
2. **`|| { ... }`** — замыкание **без `move`** → **заимствует** `data`.
3. **`&data`** — **разделяется** между потоками.
4. **`RefCell<i32>: !Sync`** → **компилятор отказывается**.

### Ошибка

```
error[E0277]: `RefCell<i32>` cannot be shared between threads safely
  --> src/main.rs:6:17
   |
6  |         s.spawn(|| {
   |                 ^^ `RefCell<i32>` cannot be shared between threads safely
   |
   = help: the trait `Sync` is not implemented for `RefCell<i32>`
```

### Почему `RefCell: !Sync`

- **Внутренние счётчики заимствования** (`borrow flags`) — **неатомарные** (`Cell<BorrowFlag>`).
- Если **два потока** одновременно вызовут `borrow()` или `borrow_mut()` — **гонка данных** → **UB**.

## Ключевое различие

| | `thread::spawn(move)` | `thread::scope` |
|---|---|---|
| **Замыкание** | `move` — **перемещает** | без `move` — **заимствует** |
| **Требует** | **`Send`** | **`Sync`** |
| **`RefCell`** | ✅ Работает | ❌ Не работает |
| **`Mutex`** | ✅ Работает | ✅ Работает |

**Правило:**

- **`thread::spawn`** — **перемещает** → нужен **`Send`**.
- **`thread::scope`** — **разделяет ссылки** → нужен **`Sync`**.

## Сводная таблица

| Тип | `Send` | `Sync` | `thread::spawn(move)` | `thread::scope` |
|---|---|---|---|---|
| `i32` | ✅ | ✅ | ✅ | ✅ |
| `String` | ✅ | ✅ | ✅ | ✅ |
| `Arc<Vec<i32>>` | ✅ | ✅ | ✅ | ✅ |
| `RefCell<i32>` | ✅ | ❌ | ✅ | ❌ |
| `Cell<i32>` | ✅ | ❌ | ✅ | ❌ |
| `Rc<i32>` | ❌ | ❌ | ❌ | ❌ |
| `Mutex<i32>` | ✅ | ✅ | ✅ | ✅ |
| `*mut i32` | ❌ | ❌ | ❌ | ❌ |

## Разбор утверждений из комментариев

### 1. «`RefCell<i32>` является `Send`»

✅ **Верно.** `i32: Send`, `RefCell` не содержит `!Send`-полей → `RefCell<i32>: Send`.

### 2. «`Send` — auto trait, компилятор выводит по полям»

✅ **Верно.** `Send` и `Sync` — **auto traits**.

### 3. «`RefCell<i32>` — `!Sync`»

✅ **Верно.** Неатомарные borrow flags → **гонка** → **`!Sync`**.

### 4. «Сырые указатели не реализуют `Send`/`Sync`»

✅ **Верно.** `*const T`, `*mut T` — **`!Send + !Sync`**.

### 5. «Если заменить `Arc` на `Rc`, компилятор откажется собирать»

✅ **Верно.** `Rc<T>: !Send + !Sync` → `thread::spawn` **не примет**.

## Полный рабочий пример

```rust
use std::sync::Arc;
use std::thread;
use std::cell::RefCell;

fn main() {
    // === 1. Arc ===
    let v = Arc::new(vec![1, 2, 3]);

    let mut hands = vec![];
    for i in 0..5 {
        hands.push(thread::spawn({
            let v_clone = v.clone();
            move || {
                println!("Thread {}: {:?}", i, *v_clone);
            }
        }));
    }
    for h in hands {
        h.join().unwrap();
    }

    // === 2. RefCell + move ===
    let data = RefCell::new(10);
    thread::spawn(move || {
        *data.borrow_mut() += 10;
        println!("val: {}", *data.borrow());   // 20
    })
    .join()
    .unwrap();

    // === 3. RefCell + scope — НЕ КОМПИЛИРУЕТСЯ ===
    // let data = RefCell::new(10);
    // thread::scope(|s| {
    //     s.spawn(|| {
    //         println!("{}", *data.borrow());
    //     });
    // });
}
```

## Сводная таблица

| Аспект | `Send` | `Sync` |
|---|---|---|
| **Что означает** | Перемещение между потоками | Разделение `&T` между потоками |
| **Auto trait** | ✅ Да | ✅ Да |
| **Связь** | — | `T: Sync ⇔ &T: Send` |
| **`thread::spawn(move)`** | Требует `Send` | — |
| **`thread::scope`** | — | Требует `Sync` |
| **`Rc`** | ❌ | ❌ |
| **`Arc`** | ✅ (если `T: Send + Sync`) | ✅ (если `T: Send + Sync`) |
| **`RefCell`** | ✅ | ❌ |
| **`Cell`** | ✅ | ❌ |
| **`Mutex`** | ✅ | ✅ |
| **`*mut T`** | ❌ | ❌ |

## Итог

- **`Send`** — значение можно **переместить** в другой поток.
- **`Sync`** — `&T` можно **разделить** между потоками.
- **Оба — auto traits** — компилятор **выводит** по полям.
- **`Arc<Vec<i32>>`** — `Send + Sync` → **5 потоков** читают **одновременно**.
- **`RefCell<i32>`** — `Send`, но `!Sync`:
  - можно **переместить** в поток (`thread::spawn(move)`);
  - нельзя **разделить** между потоками (`thread::scope`).
- **`Rc`** — `!Send + !Sync` (неатомарный счётчик).
- **Сырые указатели** — `!Send + !Sync`.
- **`thread::spawn(move)`** требует **`Send`**.
- **`thread::scope`** требует **`Sync`**.
- **Правило:** `RefCell` — для **однопоточного** interior mutability; для **многопоточного** — `Mutex`/`RwLock`.
