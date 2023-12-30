use annotations::{pre, post, invariant};

/// Calculates the factorial of a number.
/// 
/// # Arguments
/// 
/// * `n` - A non-negative integer whose factorial is to be calculated.
///
/// # Returns
/// 
/// The factorial of `n`.
pub fn factorial(n: i32) -> i32 {
    pre!("n >= 0");
    post!("result >= 1");
    
    let mut result = 1;
    let mut counter = 1;

    invariant!("result == factorial(counter - 1) && counter <= n + 1");
    while counter <= n {
        result *= counter;
        counter += 1;
    }

    if n == 0 {
        result = 1; // Special case for 0! which is 1.
    }
    else if n < 0 {
        result = 0; // Handling negative inputs.
    }

    result
}

fn main() {
    let num = 5;
    let factorial_result = factorial(num);
    println!("Factorial of {} is {}", num, factorial_result);
}
