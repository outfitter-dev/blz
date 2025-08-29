# Async Patterns for AI Agents

## Overview

This guide addresses common async/await pain points that AI agents encounter when working with Rust code. It provides concrete patterns, templates, and anti-patterns to help agents write correct async code.

## Common Agent Pain Points

### 1. Send + Sync + 'static Bounds

**Problem**: Agents often struggle with trait bounds required for spawning async tasks.

**Solution**: Use owned data and Arc for shared state.

```rust
// ❌ BAD: Borrowing across task boundary
pub async fn spawn_worker(data: &SomeData) -> tokio::task::JoinHandle<Result<()>> {
    tokio::spawn(async move {
        // ERROR: borrowed data doesn't live long enough
        process_data(data).await
    })
}

// ✅ GOOD: Own the data or use Arc
pub async fn spawn_worker(data: Arc<SomeData>) -> tokio::task::JoinHandle<Result<()>> {
    tokio::spawn(async move {
        // OK: Arc is Send + Sync + 'static
        process_data(&data).await
    })
}

// ✅ GOOD: Clone small data
pub async fn spawn_worker(data: SomeSmallData) -> tokio::task::JoinHandle<Result<()>>
where
    SomeSmallData: Clone + Send + 'static,
{
    tokio::spawn(async move {
        // OK: owned data is 'static
        process_data(data).await
    })
}
```

### 2. Borrowing Across .await Points

**Problem**: Holding references across `.await` points causes "future cannot be sent between threads safely" errors.

**Anti-Pattern**:
```rust
// ❌ BAD: Borrowing across await
pub async fn bad_pattern(cache: &mut Cache) -> Result<()> {
    let item = cache.get_mut("key"); // Mutable borrow starts here
    some_async_operation().await;    // ERROR: borrow held across await
    item.update();
    Ok(())
}
```

**Solution**: Clone or restructure to avoid holding references:
```rust
// ✅ GOOD: Clone the data
pub async fn good_pattern(cache: &mut Cache) -> Result<()> {
    let item = cache.get("key").cloned(); // Clone the data
    some_async_operation().await;
    
    if let Some(mut item) = item {
        item.update();
        cache.insert("key", item);
    }
    Ok(())
}

// ✅ GOOD: Restructure to avoid borrowing
pub async fn better_pattern(cache: Arc<Mutex<Cache>>) -> Result<()> {
    let item = {
        let cache = cache.lock().await;
        cache.get("key").cloned()
    }; // Lock released here
    
    some_async_operation().await;
    
    if let Some(mut item) = item {
        item.update();
        let mut cache = cache.lock().await;
        cache.insert("key", item);
    }
    Ok(())
}
```

### 3. Task Spawning Templates

**Basic Task Spawning**:
```rust
use tokio::task::JoinHandle;
use std::sync::Arc;

/// Spawn a task with owned data
pub fn spawn_task<F, T>(future: F) -> JoinHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    tokio::spawn(future)
}

/// Spawn multiple tasks with shared state
pub async fn spawn_workers<T: Clone + Send + Sync + 'static>(
    shared_data: Arc<T>,
    work_items: Vec<String>,
) -> Vec<JoinHandle<Result<String>>> {
    work_items
        .into_iter()
        .map(|item| {
            let data = Arc::clone(&shared_data);
            tokio::spawn(async move {
                process_work_item(&data, &item).await
            })
        })
        .collect()
}

async fn process_work_item<T>(data: &T, item: &str) -> Result<String> {
    // Your async work here
    Ok(format!("processed: {}", item))
}
```

### 4. Arc vs Rc Pattern

**Use Arc for async, Rc for single-threaded**:

```rust
use std::sync::Arc;
use std::rc::Rc;

// ✅ GOOD: Arc for async/multi-threaded
pub async fn async_work(data: Arc<ExpensiveData>) -> Result<()> {
    let data_clone = Arc::clone(&data);
    
    tokio::spawn(async move {
        // Can move Arc across task boundary
        process_async(&data_clone).await
    });
    
    Ok(())
}

// ✅ GOOD: Rc for single-threaded (but not with async tasks)
pub fn sync_work(data: Rc<ExpensiveData>) -> Result<()> {
    // Use Rc for single-threaded code only
    process_sync(&data)
}
```

### 5. Channel Patterns

**Producer-Consumer with Channels**:
```rust
use tokio::sync::mpsc;

pub async fn channel_pattern() -> Result<()> {
    let (tx, mut rx) = mpsc::channel(100);
    
    // Producer task
    let producer = tokio::spawn(async move {
        for i in 0..10 {
            if tx.send(format!("item {}", i)).await.is_err() {
                break; // Receiver dropped
            }
        }
    });
    
    // Consumer task
    let consumer = tokio::spawn(async move {
        while let Some(item) = rx.recv().await {
            println!("Processing: {}", item);
        }
    });
    
    // Wait for completion
    let _ = tokio::try_join!(producer, consumer)?;
    Ok(())
}
```

## Memory Management in Async

### 1. Avoiding Memory Leaks

```rust
// ❌ BAD: Potential memory leak with cyclic references
struct BadWorker {
    data: Arc<Mutex<WorkerData>>,
    handle: Option<JoinHandle<()>>,
}

// ✅ GOOD: Use weak references or drop handles explicitly
struct GoodWorker {
    data: Arc<Mutex<WorkerData>>,
    _handle: JoinHandle<()>, // Will be aborted on drop
}

impl GoodWorker {
    pub fn new() -> Self {
        let data = Arc::new(Mutex::new(WorkerData::new()));
        let data_weak = Arc::downgrade(&data);
        
        let handle = tokio::spawn(async move {
            while let Some(data) = data_weak.upgrade() {
                // Work with data
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
        
        Self { data, _handle: handle }
    }
}

impl Drop for GoodWorker {
    fn drop(&mut self) {
        self._handle.abort(); // Ensure task is cancelled
    }
}
```

### 2. Resource Cleanup

```rust
use tokio::sync::Mutex;
use std::sync::Arc;

pub struct ResourceManager {
    resources: Arc<Mutex<Vec<Resource>>>,
}

impl ResourceManager {
    pub async fn cleanup_task(resources: Arc<Mutex<Vec<Resource>>>) {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        
        loop {
            interval.tick().await;
            
            let mut resources = resources.lock().await;
            resources.retain(|r| !r.should_cleanup());
            
            // Resource cleanup happens here via Drop
        }
    }
}
```

## Error Handling in Async

### 1. Propagating Errors from Tasks

```rust
// ❌ BAD: Losing error information
pub async fn bad_error_handling() -> Result<()> {
    let handle = tokio::spawn(async {
        fallible_operation().await // Error lost!
    });
    
    handle.await.unwrap(); // Only gets JoinError, not original error
    Ok(())
}

// ✅ GOOD: Preserve error information
pub async fn good_error_handling() -> Result<()> {
    let handle: JoinHandle<Result<()>> = tokio::spawn(async {
        fallible_operation().await // Returns Result
    });
    
    match handle.await {
        Ok(result) => result, // Propagate original error
        Err(join_error) => {
            if join_error.is_cancelled() {
                return Err(Error::TaskCancelled);
            } else if join_error.is_panic() {
                return Err(Error::TaskPanicked);
            } else {
                return Err(Error::TaskFailed);
            }
        }
    }
}

async fn fallible_operation() -> Result<()> {
    // Your fallible async operation
    Ok(())
}
```

### 2. Timeout Patterns

```rust
use tokio::time::{timeout, Duration};

pub async fn with_timeout<F, T>(future: F, secs: u64) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    match timeout(Duration::from_secs(secs), future).await {
        Ok(result) => result,
        Err(_) => Err(Error::Timeout),
    }
}

// Usage
pub async fn search_with_timeout(query: &str) -> Result<SearchResults> {
    with_timeout(
        perform_search(query),
        30 // 30 second timeout
    ).await
}
```

## Async Testing Patterns

### 1. Testing Async Functions

```rust
#[tokio::test]
async fn test_async_function() {
    let result = async_function("test_input").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_concurrent_operations() {
    let data = Arc::new(TestData::new());
    
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let data = Arc::clone(&data);
            tokio::spawn(async move {
                concurrent_operation(&data, i).await
            })
        })
        .collect();
    
    let results = futures::future::try_join_all(handles).await.unwrap();
    
    // Verify all operations completed successfully
    assert_eq!(results.len(), 10);
    assert!(results.iter().all(|r| r.is_ok()));
}
```

## Quick Reference Templates

### Spawn Task with Shared State
```rust
let shared_data = Arc::new(MyData::new());
let data_clone = Arc::clone(&shared_data);

let handle = tokio::spawn(async move {
    // Use data_clone here
    process_data(&data_clone).await
});
```

### Producer-Consumer
```rust
let (tx, mut rx) = tokio::sync::mpsc::channel(100);

// Producer
tokio::spawn(async move {
    for item in items {
        let _ = tx.send(item).await;
    }
});

// Consumer
tokio::spawn(async move {
    while let Some(item) = rx.recv().await {
        process_item(item).await;
    }
});
```

### Timeout Any Operation
```rust
use tokio::time::{timeout, Duration};

let result = timeout(
    Duration::from_secs(30),
    your_async_operation()
).await??; // First ? for timeout, second ? for operation result
```

## Anti-Patterns to Avoid

1. **Never** hold `MutexGuard` across `.await` points
2. **Never** use `std::sync::Mutex` with async code (use `tokio::sync::Mutex`)  
3. **Never** call `.unwrap()` on `JoinHandle::await` without checking join errors
4. **Never** forget to handle task cancellation in long-running tasks
5. **Never** use blocking operations inside async functions (use async alternatives)

## When to Use What

| Scenario | Use | Example |
|----------|-----|---------|
| Shared immutable state | `Arc<T>` | Configuration, read-only data |
| Shared mutable state | `Arc<Mutex<T>>` | Cache, counters |
| Communication | Channels | Producer-consumer, work queues |
| Small owned data | Clone | IDs, small strings |
| One-shot values | `oneshot` channels | Request-response |

Remember: The async system is designed to be helpful. When you get a compiler error, it's usually preventing a real bug!