use annotations::{pre, post, invariant};

/// Calculates the sum of the first `n` Fibonacci numbers.
/// 
/// # Arguments
/// 
/// * `n` - A non-negative integer indicating how many Fibonacci numbers to sum.
///
/// # Returns
/// 
/// The sum of the first `n` Fibonacci numbers.
pub fn fibonacci_sum(n: i32) -> i32 {
    pre!("n >= 0");
    post!("sum >= 0");

    let mut fib = vec![0, 1];
    let mut next = 1;
    let mut counter = 2;

    invariant!("fib.len() == counter && counter <= n + 1");
    while counter <= n {
        let new_fib = fib[counter - 1] + fib[counter - 2];
        fib.push(new_fib);
        counter += 1;
    }

    let mut sum = 0;
    invariant!("sum >= 0");
    for fib_number in fib.iter().take(n as usize) {
        sum += fib_number;
    }

    sum
}

fn main() {
    let num = 5;
    let sum_result = fibonacci_sum(num);
    println!("Sum of the first {} Fibonacci numbers is {}", num, sum_result);
}
