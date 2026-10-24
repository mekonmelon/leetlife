//! Seed data: a curated catalogue of interview-classic problems and the shop.
//! Seeding is idempotent (upserts keyed by slug / item id), so it runs on every
//! start and picks up catalogue changes without a migration.
use super::repo::{self, NewProblem};
use crate::error::Result;
use crate::models::Difficulty;
use sqlx::SqliteConnection;

pub struct SeedProblem {
    pub id: i64,
    pub slug: &'static str,
    pub title: &'static str,
    pub difficulty: Difficulty,
    pub topic: &'static str,
    pub importance: f64,
}

const fn p(id: i64, slug: &'static str, title: &'static str, difficulty: Difficulty, topic: &'static str, importance: f64) -> SeedProblem {
    SeedProblem { id, slug, title, difficulty, topic, importance }
}

use Difficulty::{Easy, Hard, Medium};

pub const PROBLEMS: &[SeedProblem] = &[
    // Arrays & Hashing
    p(1, "two-sum", "Two Sum", Easy, "Arrays", 1.0),
    p(217, "contains-duplicate", "Contains Duplicate", Easy, "Arrays", 0.7),
    p(242, "valid-anagram", "Valid Anagram", Easy, "Strings", 0.7),
    p(49, "group-anagrams", "Group Anagrams", Medium, "Strings", 0.8),
    p(347, "top-k-frequent-elements", "Top K Frequent Elements", Medium, "Heap", 0.8),
    p(238, "product-of-array-except-self", "Product of Array Except Self", Medium, "Arrays", 0.9),
    p(128, "longest-consecutive-sequence", "Longest Consecutive Sequence", Medium, "Arrays", 0.8),
    p(53, "maximum-subarray", "Maximum Subarray", Medium, "Arrays", 0.9),
    p(121, "best-time-to-buy-and-sell-stock", "Best Time to Buy and Sell Stock", Easy, "Arrays", 0.9),
    p(36, "valid-sudoku", "Valid Sudoku", Medium, "Arrays", 0.5),
    p(271, "encode-and-decode-strings", "Encode and Decode Strings", Medium, "Strings", 0.5),
    // Two pointers / sliding window
    p(125, "valid-palindrome", "Valid Palindrome", Easy, "Two Pointers", 0.7),
    p(15, "3sum", "3Sum", Medium, "Two Pointers", 0.9),
    p(11, "container-with-most-water", "Container With Most Water", Medium, "Two Pointers", 0.8),
    p(42, "trapping-rain-water", "Trapping Rain Water", Hard, "Two Pointers", 0.8),
    p(3, "longest-substring-without-repeating-characters", "Longest Substring Without Repeating Characters", Medium, "Sliding Window", 0.9),
    p(424, "longest-repeating-character-replacement", "Longest Repeating Character Replacement", Medium, "Sliding Window", 0.6),
    p(76, "minimum-window-substring", "Minimum Window Substring", Hard, "Sliding Window", 0.7),
    p(567, "permutation-in-string", "Permutation in String", Medium, "Sliding Window", 0.6),
    p(239, "sliding-window-maximum", "Sliding Window Maximum", Hard, "Sliding Window", 0.6),
    // Stack
    p(20, "valid-parentheses", "Valid Parentheses", Easy, "Stack", 0.9),
    p(155, "min-stack", "Min Stack", Medium, "Stack", 0.7),
    p(150, "evaluate-reverse-polish-notation", "Evaluate Reverse Polish Notation", Medium, "Stack", 0.6),
    p(739, "daily-temperatures", "Daily Temperatures", Medium, "Stack", 0.7),
    p(84, "largest-rectangle-in-histogram", "Largest Rectangle in Histogram", Hard, "Stack", 0.6),
    // Binary search
    p(704, "binary-search", "Binary Search", Easy, "Binary Search", 0.8),
    p(33, "search-in-rotated-sorted-array", "Search in Rotated Sorted Array", Medium, "Binary Search", 0.9),
    p(153, "find-minimum-in-rotated-sorted-array", "Find Minimum in Rotated Sorted Array", Medium, "Binary Search", 0.7),
    p(875, "koko-eating-bananas", "Koko Eating Bananas", Medium, "Binary Search", 0.6),
    p(4, "median-of-two-sorted-arrays", "Median of Two Sorted Arrays", Hard, "Binary Search", 0.7),
    // Linked list
    p(206, "reverse-linked-list", "Reverse Linked List", Easy, "Linked List", 1.0),
    p(21, "merge-two-sorted-lists", "Merge Two Sorted Lists", Easy, "Linked List", 0.9),
    p(141, "linked-list-cycle", "Linked List Cycle", Easy, "Linked List", 0.8),
    p(143, "reorder-list", "Reorder List", Medium, "Linked List", 0.6),
    p(19, "remove-nth-node-from-end-of-list", "Remove Nth Node From End of List", Medium, "Linked List", 0.7),
    p(146, "lru-cache", "LRU Cache", Medium, "Linked List", 1.0),
    p(23, "merge-k-sorted-lists", "Merge k Sorted Lists", Hard, "Heap", 0.8),
    // Trees
    p(226, "invert-binary-tree", "Invert Binary Tree", Easy, "Trees", 0.8),
    p(104, "maximum-depth-of-binary-tree", "Maximum Depth of Binary Tree", Easy, "Trees", 0.8),
    p(100, "same-tree", "Same Tree", Easy, "Trees", 0.6),
    p(235, "lowest-common-ancestor-of-a-binary-search-tree", "Lowest Common Ancestor of a BST", Medium, "Trees", 0.7),
    p(102, "binary-tree-level-order-traversal", "Binary Tree Level Order Traversal", Medium, "Trees", 0.9),
    p(98, "validate-binary-search-tree", "Validate Binary Search Tree", Medium, "Trees", 0.9),
    p(230, "kth-smallest-element-in-a-bst", "Kth Smallest Element in a BST", Medium, "Trees", 0.6),
    p(105, "construct-binary-tree-from-preorder-and-inorder-traversal", "Construct Binary Tree from Preorder and Inorder", Medium, "Trees", 0.7),
    p(124, "binary-tree-maximum-path-sum", "Binary Tree Maximum Path Sum", Hard, "Trees", 0.7),
    p(297, "serialize-and-deserialize-binary-tree", "Serialize and Deserialize Binary Tree", Hard, "Trees", 0.7),
    // Tries
    p(208, "implement-trie-prefix-tree", "Implement Trie (Prefix Tree)", Medium, "Tries", 0.8),
    p(211, "design-add-and-search-words-data-structure", "Design Add and Search Words", Medium, "Tries", 0.6),
    p(212, "word-search-ii", "Word Search II", Hard, "Tries", 0.6),
    // Heap
    p(703, "kth-largest-element-in-a-stream", "Kth Largest Element in a Stream", Easy, "Heap", 0.6),
    p(215, "kth-largest-element-in-an-array", "Kth Largest Element in an Array", Medium, "Heap", 0.9),
    p(295, "find-median-from-data-stream", "Find Median from Data Stream", Hard, "Heap", 0.7),
    // Backtracking
    p(78, "subsets", "Subsets", Medium, "Backtracking", 0.8),
    p(39, "combination-sum", "Combination Sum", Medium, "Backtracking", 0.8),
    p(46, "permutations", "Permutations", Medium, "Backtracking", 0.8),
    p(79, "word-search", "Word Search", Medium, "Backtracking", 0.7),
    p(51, "n-queens", "N-Queens", Hard, "Backtracking", 0.6),
    // Graphs
    p(200, "number-of-islands", "Number of Islands", Medium, "Graphs", 1.0),
    p(133, "clone-graph", "Clone Graph", Medium, "Graphs", 0.8),
    p(417, "pacific-atlantic-water-flow", "Pacific Atlantic Water Flow", Medium, "Graphs", 0.6),
    p(207, "course-schedule", "Course Schedule", Medium, "Graphs", 0.9),
    p(210, "course-schedule-ii", "Course Schedule II", Medium, "Graphs", 0.8),
    p(994, "rotting-oranges", "Rotting Oranges", Medium, "Graphs", 0.7),
    p(323, "number-of-connected-components-in-an-undirected-graph", "Number of Connected Components", Medium, "Graphs", 0.7),
    p(261, "graph-valid-tree", "Graph Valid Tree", Medium, "Graphs", 0.6),
    p(127, "word-ladder", "Word Ladder", Hard, "Graphs", 0.7),
    p(743, "network-delay-time", "Network Delay Time", Medium, "Graphs", 0.7),
    p(269, "alien-dictionary", "Alien Dictionary", Hard, "Graphs", 0.7),
    // Dynamic programming
    p(70, "climbing-stairs", "Climbing Stairs", Easy, "Dynamic Programming", 0.9),
    p(198, "house-robber", "House Robber", Medium, "Dynamic Programming", 0.8),
    p(213, "house-robber-ii", "House Robber II", Medium, "Dynamic Programming", 0.6),
    p(5, "longest-palindromic-substring", "Longest Palindromic Substring", Medium, "Dynamic Programming", 0.8),
    p(322, "coin-change", "Coin Change", Medium, "Dynamic Programming", 1.0),
    p(300, "longest-increasing-subsequence", "Longest Increasing Subsequence", Medium, "Dynamic Programming", 0.8),
    p(139, "word-break", "Word Break", Medium, "Dynamic Programming", 0.8),
    p(1143, "longest-common-subsequence", "Longest Common Subsequence", Medium, "Dynamic Programming", 0.8),
    p(62, "unique-paths", "Unique Paths", Medium, "Dynamic Programming", 0.7),
    p(72, "edit-distance", "Edit Distance", Medium, "Dynamic Programming", 0.7),
    p(152, "maximum-product-subarray", "Maximum Product Subarray", Medium, "Dynamic Programming", 0.7),
    p(91, "decode-ways", "Decode Ways", Medium, "Dynamic Programming", 0.6),
    p(10, "regular-expression-matching", "Regular Expression Matching", Hard, "Dynamic Programming", 0.5),
    // Greedy / intervals
    p(55, "jump-game", "Jump Game", Medium, "Greedy", 0.7),
    p(134, "gas-station", "Gas Station", Medium, "Greedy", 0.5),
    p(57, "insert-interval", "Insert Interval", Medium, "Intervals", 0.7),
    p(56, "merge-intervals", "Merge Intervals", Medium, "Intervals", 1.0),
    p(435, "non-overlapping-intervals", "Non-overlapping Intervals", Medium, "Intervals", 0.7),
    p(252, "meeting-rooms", "Meeting Rooms", Easy, "Intervals", 0.7),
    p(253, "meeting-rooms-ii", "Meeting Rooms II", Medium, "Intervals", 0.9),
    // Bits / math
    p(191, "number-of-1-bits", "Number of 1 Bits", Easy, "Bit Manipulation", 0.5),
    p(338, "counting-bits", "Counting Bits", Easy, "Bit Manipulation", 0.5),
    p(190, "reverse-bits", "Reverse Bits", Easy, "Bit Manipulation", 0.4),
    p(268, "missing-number", "Missing Number", Easy, "Bit Manipulation", 0.6),
    p(48, "rotate-image", "Rotate Image", Medium, "Math", 0.7),
    p(54, "spiral-matrix", "Spiral Matrix", Medium, "Math", 0.7),
    p(73, "set-matrix-zeroes", "Set Matrix Zeroes", Medium, "Math", 0.6),
    p(202, "happy-number", "Happy Number", Easy, "Math", 0.4),
];

pub async fn seed_all(c: &mut SqliteConnection) -> Result<()> {
    for sp in PROBLEMS {
        repo::upsert_problem(
            c,
            &NewProblem { slug: sp.slug, title: sp.title, difficulty: sp.difficulty, topic: sp.topic, importance: sp.importance, leetcode_id: Some(sp.id) },
        )
        .await?;
    }
    for item in crate::game::catalog() {
        repo::upsert_item(c, &item).await?;
    }
    Ok(())
}
