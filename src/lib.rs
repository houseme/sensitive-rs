/// This is the main file of the library.
/// It contains the main logic of the library.
/// # Examples
/// ```
/// use sensitive_rs::Filter;
/// let mut filter = Filter::new();
/// filter.update_noise_pattern(r"[\|\s&%$@*]+");
/// filter.add_words(&["apple", "app", "banana"]);
/// assert_eq!(filter.filter("I |have& %an$ @apple* and a banana"), "I have an and a");
/// ```
/// # Features
/// - Add words to the filter
/// - Remove words from the filter
/// - Replace words in a string
/// - Filter words from a string
/// - Update noise pattern
/// - Find words in a string
/// - Find words in a string with their positions
/// - Find words in a string with their positions and lengths
/// - Find words in a string with their positions, lengths and values
/// - Find words in a string with their positions, lengths, values and noise
/// - Find words in a string with their positions, lengths, values, noise and noise positions
///
/// # Performance
/// - The filter is implemented using a trie data structure.
/// - The filter is case-insensitive.
/// - The filter is fast and efficient.
/// - The filter is thread-safe.
/// - The filter is memory-efficient.
///
/// # Safety
/// - The filter is safe to use.
/// - The filter is safe to share across threads.
/// - The filter is safe to use in a multi-threaded environment.
/// - The filter is safe to use in a multi-threaded environment with multiple filters.
///
/// # Errors
/// - The filter returns an error if the noise pattern is invalid.
/// - The filter returns an error if the noise pattern is invalid and the noise pattern is updated.
/// - The filter returns an error if the noise pattern is invalid and the noise pattern is updated with an invalid noise pattern.
///
/// # Panics
/// - The filter panics if the noise pattern is invalid.
/// - The filter panics if the noise pattern is invalid and the noise pattern is updated.
///
/// Each module contains a set of functions that can be used to interact with the filter.
mod filter;
mod trie;
