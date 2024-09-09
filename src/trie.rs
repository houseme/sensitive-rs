use std::collections::{HashMap, VecDeque};

#[derive(Debug)]
pub struct Trie {
    root: Node,
}

#[derive(Debug, Clone)]
pub struct Node {
    is_root_node: bool,
    is_path_end: bool,
    character: char,
    children: HashMap<char, Node>,
    failure: Option<Box<Node>>,
    parent: Option<Box<Node>>,
    depth: usize,
}

impl Trie {
    /// Creates a new trie.
    /// A trie is a tree-like data structure that stores a dynamic set of strings.
    /// A trie has a root node that represents the starting point of the trie.
    pub fn new() -> Self {
        Trie { root: Node::new_root() }
    }

    /// Adds words to the trie.
    /// Words are added to the trie by splitting them into characters.
    /// Each character is added to the trie as a node.
    pub fn add(&mut self, words: &[&str]) {
        for word in words {
            self.add_word(word);
        }
    }

    /// Adds a word to the trie.
    /// A word is added to the trie by splitting it into characters.
    fn add_word(&mut self, word: &str) {
        let mut current = &mut self.root;
        for (position, c) in word.chars().enumerate() {
            if !current.children.contains_key(&c) {
                let new_node = Node::new(c, position + 1, Some(current));
                current.children.insert(c, new_node);
            }
            current = current.children.get_mut(&c).unwrap();
        }
        current.is_path_end = true;
    }

    /// Builds failure links in the trie.
    /// Failure links are used to find the longest suffix that is also a prefix.
    /// Failure links are used to find the next node to visit when a character is not found.
    pub fn build_failure_links(&mut self) {
        let root_clone = self.root.clone();
        let mut queue = VecDeque::new();
        for child in self.root.children.values_mut() {
            queue.push_back(child);
        }

        while let Some(node) = queue.pop_front() {
            let mut pointer = node.parent.as_ref().map(|p| p.as_ref()).unwrap_or(&root_clone);
            let mut link = None;

            while link.is_none() {
                if pointer.is_root_node() {
                    link = Some(pointer);
                    break;
                }
                link = pointer.failure.as_ref().and_then(|fail| fail.children.get(&node.character));
                pointer = pointer.failure.as_ref().map(|fail| fail.as_ref()).unwrap_or(&root_clone);
            }

            node.failure = link.map(|l| Box::new(l.clone()));

            for child in node.children.values_mut() {
                queue.push_back(child);
            }
        }
    }

    /// Replaces words in the text with a character.
    /// Words are replaced by traversing the trie and replacing characters.
    pub fn replace(&self, text: &str, character: char) -> String {
        let mut node = &self.root;
        let mut runes: Vec<char> = text.chars().collect();

        for position in 0..runes.len() {
            node = self.next(node, runes[position]).unwrap_or_else(|| self.fail(node, runes[position]));
            self.replace_node(node, &mut runes, position, character);
        }

        runes.iter().collect()
    }

    /// Filters words from the text.
    /// Words are filtered by traversing the trie and removing characters.
    /// Words are removed if they are found in the trie.
    pub fn filter(&self, text: &str) -> String {
        let mut parent = &self.root;
        let mut left = 0;
        let mut result_runes = Vec::new();
        let runes: Vec<char> = text.chars().collect();
        let length = runes.len();

        for position in 0..length {
            if let Some(current) = parent.children.get(&runes[position]) {
                if current.is_path_end {
                    left = position + 1;
                    parent = &self.root;
                } else {
                    parent = current;
                }
            } else {
                result_runes.push(runes[left]);
                parent = &self.root;
                left += 1;
            }
        }

        result_runes.extend_from_slice(&runes[left..]);
        result_runes.iter().collect()
    }

    /// Validates the text.
    /// The text is validated by traversing the trie and checking if the text contains any words.
    /// The text is validated by checking if the text contains any words.
    pub fn validate(&self, text: &str) -> (bool, String) {
        let mut node = &self.root;
        let runes: Vec<char> = text.chars().collect();

        for position in 0..runes.len() {
            node = self.next(node, runes[position]).unwrap_or_else(|| self.fail(node, runes[position]));
            if let Some(first) = self.first_output(node, &runes, position) {
                return (false, first);
            }
        }

        (true, String::new())
    }

    /// Validates the text with a wildcard.
    /// The text is validated by traversing the trie and checking if the text contains any words.
    /// The text is validated by checking if the text contains any words.
    pub fn validate_with_wildcard(&self, text: &str, wildcard: char) -> (bool, String) {
        let runes: Vec<char> = text.chars().collect();
        for curl in 0..runes.len() {
            let mut pattern = String::new();
            if self.dfs(&runes, &self.root, curl, wildcard, &mut pattern) {
                return (false, pattern);
            }
        }
        (true, String::new())
    }

    fn dfs(&self, runes: &[char], parent: &Node, curl: usize, wildcard: char, pattern: &mut String) -> bool {
        if parent.is_path_end {
            return true;
        }
        if curl >= runes.len() {
            return false;
        }

        if let Some(current) = parent.children.get(&runes[curl]) {
            if self.dfs(runes, current, curl + 1, wildcard, pattern) {
                return true;
            }
        }

        if let Some(current1) = parent.children.get(&wildcard) {
            if self.dfs(runes, current1, curl + 1, wildcard, pattern) {
                return true;
            }
            if let Some(current2) = current1.children.get(&runes[curl]) {
                if self.dfs(runes, current2, curl + 1, wildcard, pattern) {
                    return true;
                }
            }
        }

        false
    }

    /// Finds words in the text.
    /// Words are found by traversing the trie and checking if the text contains any words.
    /// Words are found by checking if the text contains any words.
    /// The first word found is returned.
    pub fn find_in(&self, text: &str) -> (bool, String) {
        let (validated, first) = self.validate(text);
        (!validated, first)
    }

    /// Finds all words in the text.
    /// Words are found by traversing the trie and checking if the text contains any words.
    /// Words are found by checking if the text contains any words.
    /// All words found are returned.
    pub fn find_all(&self, text: &str) -> Vec<String> {
        let mut node = &self.root;
        let runes: Vec<char> = text.chars().collect();
        let mut results = Vec::new();

        for position in 0..runes.len() {
            node = self.next(node, runes[position]).unwrap_or_else(|| self.fail(node, runes[position]));
            self.output(node, &runes, position, &mut results);
        }

        results
    }

    fn next<'a>(&'a self, node: &'a Node, character: char) -> Option<&'a Node> {
        node.children.get(&character)
    }

    fn fail<'a>(&'a self, node: &'a Node, character: char) -> &'a Node {
        let mut failure = node.failure.as_ref().unwrap();
        while failure.children.get(&character).is_none() && !failure.is_root_node {
            failure = failure.failure.as_ref().unwrap();
        }
        failure.children.get(&character).unwrap_or_else(move || &self.root)
    }

    fn replace_node(&self, node: &Node, runes: &mut [char], position: usize, character: char) {
        if node.is_path_end {
            for i in (position + 1 - node.depth)..=position {
                runes[i] = character;
            }
        }
    }

    fn first_output(&self, node: &Node, runes: &[char], position: usize) -> Option<String> {
        if node.is_path_end {
            Some(runes[(position + 1 - node.depth)..=position].iter().collect())
        } else {
            None
        }
    }

    fn output(&self, node: &Node, runes: &[char], position: usize, results: &mut Vec<String>) {
        if node.is_path_end {
            results.push(runes[(position + 1 - node.depth)..=position].iter().collect());
        }
    }
}

impl Trie {
    /// Deletes a word from the trie.
    /// A word is deleted by traversing the trie and removing nodes if they are no longer necessary.
    pub fn del(&mut self, word: &str) {
        let root = &mut self.root;
        Trie::del_recursive(root, word, 0);
    }

    fn del_recursive(node: &mut Node, word: &str, depth: usize) -> bool {
        if depth == word.len() {
            if !node.is_path_end {
                return false; // Word not found
            }
            node.is_path_end = false;
            return node.children.is_empty(); // If true, delete this node
        }

        let ch = word.chars().nth(depth).unwrap();
        if let Some(child) = node.children.get_mut(&ch) {
            if Trie::del_recursive(child, word, depth + 1) {
                node.children.remove(&ch);
                return !node.is_path_end && node.children.is_empty();
            }
        }
        false
    }
}

impl Node {
    /// Creates a new node.
    /// A node is a single element in a trie.
    /// A node has a character, children, and a failure link.
    pub fn new(character: char, depth: usize, parent: Option<&Node>) -> Self {
        Node {
            is_root_node: false,
            is_path_end: false,
            character,
            children: HashMap::new(),
            failure: None,
            parent: parent.map(|p| Box::new(p.clone())),
            depth,
        }
    }

    /// Creates a new root node.
    /// A root node is a node that has no parent.
    /// A root node is the starting point of a trie.
    pub fn new_root() -> Self {
        Node {
            is_root_node: true,
            is_path_end: false,
            character: '\0',
            children: HashMap::new(),
            failure: None,
            parent: None,
            depth: 0,
        }
    }

    /// Returns true if the node is a leaf node.
    /// A leaf node is a node that has no children.
    pub fn is_leaf_node(&self) -> bool {
        self.children.is_empty()
    }
    /// Returns true if the node is a root node.
    /// A root node is a node that has no parent.
    pub fn is_root_node(&self) -> bool {
        self.is_root_node
    }
    /// Returns true if the node is the end of a path.
    /// The end of a path is a node that represents the end of a word.
    pub fn is_path_end(&self) -> bool {
        self.is_path_end
    }
    /// Sets the node as the end of a path.
    /// The end of a path is a node that represents the end of a word.
    pub fn soft_del(&mut self) {
        self.is_path_end = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trie_tree() {
        let mut tree = Trie::new();
        tree.add(&["习近平", "习大大"]);
        tree.build_failure_links();
        assert!(tree.root.children.contains_key(&'习'));
        assert_eq!(tree.replace("你好吗 我支持习大大，他的名字叫做习近平", '*'), "你好吗 我支持***，他的名字叫做***");
        assert_eq!(tree.filter("你好吗 我支持习大大，他的名字叫做习近平"), "你好吗 我支持，他的名字叫做");
    }

    // #[test]
    // fn test_trie_tree_bfs() {
    //     let mut tree = Trie::new();
    //     tree.add(&["习近平", "习大大", "共产党好"]);
    //     let mut ch = tree.bfs();
    //     let expect =
    //         vec![('习', 1), ('共', 1), ('近', 2), ('大', 2), ('产', 2), ('平', 3), ('大', 3), ('党', 3), ('好', 4)];
    //     let mut i = 0;
    //     while let Some(n) = ch.next() {
    //         assert_eq!(n.character, expect[i].0);
    //         assert_eq!(n.depth, expect[i].1);
    //         i += 1;
    //     }
    // }

    #[test]
    fn test_trie_tree_build_failure_links() {
        let mut tree = Trie::new();
        tree.add(&["he", "his", "she", "hers"]);
        tree.build_failure_links();
    }
    #[test]
    fn test_trie_add() {
        let mut trie = Trie::new();
        trie.add(&["hello", "world"]);
        assert!(trie.root.children.contains_key(&'h'));
        assert!(trie.root.children.contains_key(&'w'));
    }

    #[test]
    fn test_trie_build_failure_links() {
        let mut trie = Trie::new();
        trie.add(&["he", "she", "his", "hers"]);
        trie.build_failure_links();
        // Add assertions to verify failure links
    }

    #[test]
    fn test_trie_replace() {
        let mut trie = Trie::new();
        trie.add(&["习近平", "习大大"]);
        trie.build_failure_links();
        assert_eq!(trie.replace("你好吗 我支持习大大，他的名字叫做习近平", '*'), "你好吗 我支持***，他的名字叫做***");
    }

    #[test]
    fn test_trie_filter() {
        let mut trie = Trie::new();
        trie.add(&["习近平", "习大大"]);
        assert_eq!(trie.filter("你好吗 我支持习大大，他的名字叫做习近平"), "你好吗 我支持，他的名字叫做");
    }

    #[test]
    fn test_trie_validate() {
        let mut trie = Trie::new();
        trie.add(&["习近平", "习大大"]);
        assert_eq!(trie.validate("你好吗 我支持习大大，他的名字叫做习近平"), (false, "习大大".to_string()));
    }

    #[test]
    fn test_trie_validate_with_wildcard() {
        let mut trie = Trie::new();
        trie.add(&["习近平", "习大大"]);
        assert_eq!(
            trie.validate_with_wildcard("你好吗 我支持习*大，他的名字叫做习*平", '*'),
            (false, "习*大".to_string())
        );
    }

    #[test]
    fn test_trie_find_in() {
        let mut trie = Trie::new();
        trie.add(&["习近平", "习大大"]);
        assert_eq!(trie.find_in("你好吗 我支持习大大，他的名字叫做习近平"), (true, "习大大".to_string()));
    }

    #[test]
    fn test_trie_find_all() {
        let mut trie = Trie::new();
        trie.add(&["习近平", "习大大"]);
        assert_eq!(
            trie.find_all("你好吗 我支持习大大，他的名字叫做习近平"),
            vec!["习大大".to_string(), "习近平".to_string()]
        );
    }

    #[test]
    fn test_is_leaf_node() {
        let node = Node {
            is_root_node: false,
            is_path_end: false,
            character: 'a',
            children: HashMap::new(),
            failure: None,
            parent: None,
            depth: 0,
        };
        assert!(node.is_leaf_node());
    }

    #[test]
    fn test_is_root_node() {
        let root_node = Node::new_root();
        assert!(root_node.is_root_node());
    }

    #[test]
    fn test_is_path_end() {
        let node = Node {
            is_root_node: false,
            is_path_end: true,
            character: 'a',
            children: HashMap::new(),
            failure: None,
            parent: None,
            depth: 0,
        };
        assert!(node.is_path_end());
    }

    #[test]
    fn test_soft_del() {
        let mut node = Node {
            is_root_node: false,
            is_path_end: true,
            character: 'a',
            children: HashMap::new(),
            failure: None,
            parent: None,
            depth: 0,
        };
        node.soft_del();
        assert!(!node.is_path_end());
    }
}
