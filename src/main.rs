use std::sync::Arc ;
use std::thread ;

fn main() {
    let v = Arc::new(vec![1,2,3]) ;

    let mut hands = vec![] ;

    for i in 0..5 {
        hands.push(
            thread::spawn({
                let v_clone = v.clone() ;
                move || {
                    println!("Thread: {}, v_clone: {:?}", i, v_clone) ;
                    println!("*Thread: {}, v_clone: {:?}\n", i, *v_clone) ;
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
}
