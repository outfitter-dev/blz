# Async Rust Patterns for Agents

## Purpose
This guide helps AI agents understand and correctly implement async Rust patterns, focusing on common pitfalls and correct solutions.

## Common Async Task Spawning Patterns

### Pattern 1: Spawn with Owned Data
```rust
// ✅ Move owned data into spawned task
use tokio::spawn;

async fn process_files(files: Vec<String>) -> Result<()> {
    let mut handles = Vec::new();
    
    for file in files {  // file is moved into loop iteration
        let handle = spawn(async move {
            // file is moved into the async block
            process_single_file(file).await
        });
        handles.push(handle);
    }
    
    // Wait for all tasks
    for handle in handles {
        handle.await??;
    }
    
    Ok(())
}
```

### Pattern 2: Spawn with Arc-Wrapped Shared Data
```rust
// ✅ Share reference-counted data across tasks
use std::sync::Arc;
use tokio::spawn;

async fn concurrent_search(queries: Vec<String>, index: SearchIndex) -> Result<Vec<Results>, Box<dyn std::error::Error>> {
    let shared_index = Arc::new(index);
    let mut handles = Vec::new();
    
    for query in queries {
        let index_clone = Arc::clone(&shared_index);
        let handle = spawn(async move {
            // Both query and index_clone are moved into task
            index_clone.search(&query).await
        });
        handles.push(handle);
    }
    
    // Collect results
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await??);
    }
    
    Ok(results)
}
```

### Pattern 3: Spawn Tasks with Configuration
```rust
// ✅ Pass configuration to spawned tasks
use std::sync::Arc;

#[derive(Clone)]
struct Config {
    timeout: Duration,
    retries: u32,
}

async fn process_with_config(items: Vec<Item>, config: Config) -> Vec<Result<Output>> {
    let mut handles = Vec::new();
    
    for item in items {
        let config_clone = config.clone(); // Config implements Clone
        let handle = tokio::spawn(async move {
            process_item_with_config(item, config_clone).await
        });
        handles.push(handle);
    }
    
    // futures::future::join_all also works here
    futures::future::try_join_all(handles).await
}
```

## Anti-Patterns to Avoid

### Anti-Pattern: Borrowing Across Await
```rust
// ❌ This won't compile - borrowed data doesn't live long enough
async fn bad_example(data: &[String]) -> String {
    let first = &data[0];  // Borrow from data
    
    some_async_operation().await;  // Await point - borrow checker can't guarantee first is still valid
    
    first.clone()  // Error: first might not be valid anymore
}

// ✅ Fix 1: Don't hold borrows across await points
async fn good_example_1(data: &[String]) -> String {
    let result = data[0].clone();  // Clone before await
    
    some_async_operation().await;
    
    result
}

// ✅ Fix 2: Restructure to avoid the issue
async fn good_example_2(data: Vec<String>) -> String {  // Take ownership
    let first = data[0].clone();
    
    some_async_operation().await;
    
    first
}
```

### Anti-Pattern: Trying to Share Non-Send Types
```rust
// ❌ This won't compile - Rc is not Send
use std::rc::Rc;

async fn bad_sharing() {
    let data = Rc::new(vec![1, 2, 3]);
    let data_clone = data.clone();
    
    tokio::spawn(async move {
        // Error: Rc<Vec<i32>> is not Send
        println!("{:?}", data_clone);
    });
}

// ✅ Fix: Use Arc instead of Rc
use std::sync::Arc;

async fn good_sharing() {
    let data = Arc::new(vec![1, 2, 3]);  // Arc is Send + Sync
    let data_clone = Arc::clone(&data);
    
    tokio::spawn(async move {
        println!("{:?}", data_clone);  // Works!
    });
}
```

## Understanding Send + Sync + 'static Bounds

### Send: Can Be Moved Between Threads
```rust
// Types that implement Send can be moved to other threads
fn is_send<T: Send>() {}

is_send::<String>();     // ✅ String is Send
is_send::<Vec<i32>>();   // ✅ Vec<i32> is Send  
is_send::<Arc<String>>(); // ✅ Arc<String> is Send

// is_send::<Rc<String>>();  // ❌ Rc<String> is NOT Send
```

### Sync: Can Be Shared Between Threads (Behind Arc)
```rust
// Types that implement Sync can be shared between threads via Arc
fn is_sync<T: Sync>() {}

is_sync::<String>();     // ✅ String is Sync
is_sync::<i32>();        // ✅ i32 is Sync
is_sync::<Mutex<i32>>(); // ✅ Mutex<i32> is Sync

// is_sync::<RefCell<i32>>(); // ❌ RefCell<i32> is NOT Sync
```

### 'static: Lives for Entire Program Duration
```rust
// 'static means "no borrowed data" or "lives forever"

// ✅ These are 'static
let owned_string = String::from("hello");        // Owned data
let static_str = "hello";                        // String literal
let static_ref: &'static str = "hello";          // Static reference

// ❌ These are NOT 'static
fn example() {
    let local_string = String::from("hello");
    let borrowed = &local_string;                // Borrows from local_string
    // borrowed is not 'static because local_string will be dropped
}
```

## Making Types Work with Async

### Converting Non-'static References to Owned Values
```rust
// ❌ Can't spawn task with borrowed data
async fn bad_example(name: &str, data: &[i32]) {
    tokio::spawn(async move {
        // Error: name and data are not 'static
        process(name, data).await;
    });
}

// ✅ Convert to owned types
async fn good_example(name: &str, data: &[i32]) {
    let owned_name = name.to_string();      // &str -> String
    let owned_data = data.to_vec();         // &[i32] -> Vec<i32>
    
    tokio::spawn(async move {
        process(&owned_name, &owned_data).await;  // Now both are 'static
    });
}
```

### Working with Complex Borrowed Types
```rust
use std::borrow::Cow;

// Cow (Clone on Write) can help with borrowed vs owned data
async fn flexible_example(name: Cow<'_, str>) {
    let owned_name = name.into_owned();  // Convert to String regardless of input
    
    tokio::spawn(async move {
        process_name(owned_name).await;
    });
}

// Can call with either borrowed or owned data:
// flexible_example(Cow::Borrowed("borrowed")).await;
// flexible_example(Cow::Owned(String::from("owned"))).await;
```

## Async Error Handling Patterns

### Pattern: Propagate Errors from Spawned Tasks
```rust
async fn process_all(items: Vec<Item>) -> Result<Vec<Output>, ProcessError> {
    let handles: Vec<_> = items.into_iter().map(|item| {
        tokio::spawn(async move {
            process_item(item).await  // Returns Result<Output, ProcessError>
        })
    }).collect();
    
    let mut results = Vec::new();
    for handle in handles {
        let result = handle.await.map_err(|_| ProcessError::TaskPanic)??;
        results.push(result);
    }
    
    Ok(results)
}
```

### Pattern: Collect Errors Instead of Failing Fast
```rust
async fn process_all_collect_errors(items: Vec<Item>) -> (Vec<Output>, Vec<ProcessError>) {
    let handles: Vec<_> = items.into_iter().map(|item| {
        tokio::spawn(async move {
            process_item(item).await
        })
    }).collect();
    
    let mut successes = Vec::new();
    let mut errors = Vec::new();
    
    for handle in handles {
        match handle.await {
            Ok(Ok(output)) => successes.push(output),
            Ok(Err(error)) => errors.push(error),
            Err(_) => errors.push(ProcessError::TaskPanic),
        }
    }
    
    (successes, errors)
}
```

## Common Async Utilities

### Timeout Pattern
```rust
use tokio::time::{timeout, Duration};

async fn with_timeout<T>(
    future: impl Future<Output = T>,
    duration: Duration,
) -> Result<T, TimeoutError> {
    timeout(duration, future)
        .await
        .map_err(|_| TimeoutError::Elapsed)
}

// Usage
let result = with_timeout(
    expensive_operation(),
    Duration::from_secs(30)
).await?;
```

### Retry Pattern
```rust
async fn with_retry<T, E, F, Fut>(
    mut operation: F,
    max_attempts: usize,
    delay: Duration,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut last_error = None;
    
    for attempt in 1..=max_attempts {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                last_error = Some(error);
                if attempt < max_attempts {
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
    
    Err(last_error.unwrap())
}
```

## Memory Management in Async Code

### Avoid Large Objects in Async Blocks
```rust
// ❌ Large objects held across await points consume stack space
async fn bad_memory_usage() {
    let large_buffer = vec![0u8; 1024 * 1024];  // 1MB on stack
    
    some_async_operation().await;  // large_buffer held across await
    
    process_buffer(&large_buffer);
}

// ✅ Box large objects or scope them properly
async fn good_memory_usage() {
    {
        let large_buffer = vec![0u8; 1024 * 1024];
        process_buffer(&large_buffer);
    }  // large_buffer dropped here
    
    some_async_operation().await;  // No large objects held across await
}
```

### Use Arc for Shared Large Objects
```rust
// ✅ Share large objects via Arc instead of cloning
async fn share_large_object(large_data: Vec<u8>) {
    let shared_data = Arc::new(large_data);
    
    let mut handles = Vec::new();
    for i in 0..10 {
        let data_clone = Arc::clone(&shared_data);
        handles.push(tokio::spawn(async move {
            process_chunk(i, &data_clone).await;
        }));
    }
    
    futures::future::join_all(handles).await;
}
```

## Testing Async Code

### Pattern: Test Async Functions
```rust
#[tokio::test]
async fn test_async_function() {
    let result = async_function().await;
    assert_eq!(result, expected_value);
}

#[tokio::test]
async fn test_concurrent_operations() {
    let handle1 = tokio::spawn(operation_1());
    let handle2 = tokio::spawn(operation_2());
    
    let (result1, result2) = tokio::join!(handle1, handle2);
    
    assert!(result1.is_ok());
    assert!(result2.is_ok());
}
```

Remember: Async Rust is powerful but requires careful attention to ownership, lifetimes, and thread safety. When in doubt, prefer owned data over borrowed references in async contexts.