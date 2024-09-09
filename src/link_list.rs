#[derive(Debug)]
pub struct LinkList {
    head: Option<Box<ListNode>>,
    tail: Option<*mut ListNode>,
    count: usize,
}

#[derive(Debug)]
struct ListNode {
    value: Option<Box<dyn std::any::Any>>,
    next: Option<Box<ListNode>>,
}

impl LinkList {
    pub fn new() -> Self {
        LinkList { head: None, tail: None, count: 0 }
    }

    pub fn push(&mut self, value: Box<dyn std::any::Any>) {
        let node = Box::new(ListNode { value: Some(value), next: None });
        let raw_node = Box::into_raw(node);

        if self.head.is_none() {
            self.head = unsafe { Some(Box::from_raw(raw_node)) };
        } else {
            unsafe {
                (*self.tail.unwrap()).next = Some(Box::from_raw(raw_node));
            }
        }
        self.tail = Some(raw_node);
        self.count += 1;
    }

    pub fn pop(&mut self) -> Option<Box<dyn std::any::Any>> {
        self.head.take().map(|mut head| {
            self.head = head.next.take();
            self.count -= 1;
            head.value.take().unwrap()
        })
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_list_push_pop() {
        let mut list = LinkList::new();
        list.push(Box::new(1));
        list.push(Box::new("hello"));
        assert_eq!(list.pop().unwrap().downcast_ref::<i32>(), Some(&1));
        assert_eq!(list.pop().unwrap().downcast_ref::<&str>(), Some(&"hello"));
    }

    #[test]
    fn test_link_list_is_empty() {
        let mut list = LinkList::new();
        assert!(list.is_empty());
        list.push(Box::new(1));
        assert!(!list.is_empty());
    }
}
