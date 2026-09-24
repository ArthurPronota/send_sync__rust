use std::sync::Arc ;
use std::thread ;
use std::cell::RefCell ;

fn main() {
    // 1. Применение Arc
    let v = Arc::new(vec![1,2,3]) ;

    let mut hands = vec![] ;

    for i in 0..5 {
        hands.push(
            thread::spawn({
                let v_clone = v.clone() ;
                move || {
                    println!("Thread: {}, v_clone: {:?}", i, v_clone) ;
                    println!("Thread: {}, *v_clone: {:?}\n", i, *v_clone) ;
                }
            }) 
        ) ;
    }

    for h in hands {
        h.join().unwrap() ;
    } /* Out:
Thread: 0, v_clone: [1, 2, 3]
*Thread: 0, v_clone: [1, 2, 3]

Thread: 2, v_clone: [1, 2, 3]
*Thread: 2, v_clone: [1, 2, 3]

Thread: 1, v_clone: [1, 2, 3]
*Thread: 1, v_clone: [1, 2, 3]

Thread: 3, v_clone: [1, 2, 3]
*Thread: 3, v_clone: [1, 2, 3]

Thread: 4, v_clone: [1, 2, 3]
*Thread: 4, v_clone: [1, 2, 3]    
     */

    // ----------------------------------

    /*
      2. Передача RefCell<i32> в thread

    RefCell<i32> является Send, потому что:
     1) i32: Send — внутреннее значение можно перемещать между потоками.
     2) RefCell не содержит ничего, что делает его !Send:
         — нет Rc
         — нет сырых указателей
         — нет неатомарных счётчиков владения
     3) Send — auto trait — компилятор выводит его по полям.        
    */

    let data = RefCell::new(10) ;

    thread::spawn(move || { // <- замыкание с move поэтому перемещение data в замыкание
        *data.borrow_mut() += 10 ;
        println!("val: {}", *data.borrow()) ;
    })
    .join()
    .unwrap() ;


    // 3. Пример не работающего кода
    let data = RefCell::new(10) ;

    /*
    Почему RefCell<i32> — !Sync
    Краткий ответ:
        RefCell<i32> не является Sync, потому что его внутренние счётчики заимствования 
            (borrow flags) — неатомарные. 
        Если два потока одновременно попытаются взять borrow() или borrow_mut() на одном 
            RefCell, возникнет гонка данных (data race) → UB (undefined behavior).

    Почему это не компилируется:
      1) thread::scope создаёт scoped threads.
      2) Замыкание || { ... } заимствует data (без move).
      3) thread::scope требует, чтобы все заимствования были Sync — потому что ссылки 
           делятся между потоками.
      4) RefCell<i32>: !Sync → компилятор отказывается собирать.    
     */
    /*
    thread::scope(|s| {
        s.spawn(|| { // <- замыкание без move, значит происходит заимствование
            println!("{}", *data.borrow()) ;
        }) ;
    }) ;
    */
}
