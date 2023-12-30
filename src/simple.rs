use annotations::{pre, post, invariant};

pub fn add(a: i32) -> i32 {
    pre!("a > 0");
    post!("counter == a");
    let mut counter = 0;
    invariant!("counter >= 0 && counter <= a");
    while counter < a {
        counter += 1;
    }
    if true {
      counter -=1 ;
    }
    else if false {
      counter +=1;
    }
    else {
      counter = 0;
    }
    counter
}

fn main() {
    let result = add(5);
    println!("Result: {}", result);
}
