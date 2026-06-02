use crate::types::{Key, KeyRange, KeyValue, Value};

const MAX_HEIGHT: usize = 16;

#[derive(Debug, Clone)]
struct SkipListNode {
    key: Key,
    value: Value,
    forwards: Vec<Option<usize>>,
}

#[derive(Debug)]
pub struct SkipList {
    nodes: Vec<SkipListNode>,
    head: usize,
    height: usize,
    len: usize,
}

#[derive(Debug)]
pub struct MemTable {
    skiplist: SkipList,
}

impl Default for SkipList {
    fn default() -> Self {
        Self::new()
    }
}

impl SkipList {
    pub fn new() -> Self {
        Self {
            nodes: vec![SkipListNode {
                key: Vec::new(),
                value: Vec::new(),
                forwards: vec![None; MAX_HEIGHT],
            }],
            head: 0,
            height: 1,
            len: 0,
        }
    }

    pub fn insert(&mut self, key: Key, value: Value) {
        let mut update = [self.head; MAX_HEIGHT];
        let candidate = self.find_update_path(&key, &mut update);

        if let Some(index) = candidate
            && self.nodes[index].key == key
        {
            self.nodes[index].value = value;
            return;
        }

        let node_height = self.node_height();
        if node_height > self.height {
            update[self.height..node_height].fill(self.head);
            self.height = node_height;
        }

        let index = self.nodes.len();
        let forwards = (0..node_height)
            .map(|level| self.nodes[update[level]].forwards[level])
            .collect();

        self.nodes.push(SkipListNode {
            key,
            value,
            forwards,
        });

        for (level, previous) in update.iter().copied().enumerate().take(node_height) {
            self.nodes[previous].forwards[level] = Some(index);
        }

        self.len += 1;
    }

    pub fn delete(&mut self, key: &[u8]) -> Option<Value> {
        let mut update = [self.head; MAX_HEIGHT];
        let candidate = self.find_update_path(key, &mut update)?;

        if self.nodes[candidate].key.as_slice() != key {
            return None;
        }

        for (level, previous) in update.iter().copied().enumerate().take(self.height) {
            if self.nodes[previous].forwards[level] != Some(candidate) {
                continue;
            }

            self.nodes[previous].forwards[level] =
                self.nodes[candidate].forwards.get(level).copied().flatten();
        }

        while self.height > 1 && self.nodes[self.head].forwards[self.height - 1].is_none() {
            self.height -= 1;
        }

        self.len -= 1;
        Some(self.nodes[candidate].value.clone())
    }

    pub fn search(&self, key: &[u8]) -> Option<&Value> {
        let candidate = self.lower_bound(key)?;

        if self.nodes[candidate].key.as_slice() == key {
            Some(&self.nodes[candidate].value)
        } else {
            None
        }
    }

    pub fn scan(&self, range: KeyRange) -> Vec<KeyValue> {
        let mut index = match range.start.as_deref() {
            Some(start) => self.lower_bound(start),
            None => self.nodes[self.head].forwards[0],
        };
        let mut results = Vec::new();

        while let Some(current) = index {
            let node = &self.nodes[current];

            if range
                .end
                .as_ref()
                .is_some_and(|end| node.key.as_slice() >= end.as_slice())
            {
                break;
            }

            results.push(KeyValue {
                key: node.key.clone(),
                value: node.value.clone(),
            });
            index = node.forwards[0];
        }

        results
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn find_update_path(&self, key: &[u8], update: &mut [usize; MAX_HEIGHT]) -> Option<usize> {
        let mut current = self.head;

        for level in (0..self.height).rev() {
            while let Some(next) = self.nodes[current].forwards[level] {
                if self.nodes[next].key.as_slice() >= key {
                    break;
                }

                current = next;
            }

            update[level] = current;
        }

        self.nodes[current].forwards[0]
    }

    fn lower_bound(&self, key: &[u8]) -> Option<usize> {
        let mut current = self.head;

        for level in (0..self.height).rev() {
            while let Some(next) = self.nodes[current].forwards[level] {
                if self.nodes[next].key.as_slice() >= key {
                    break;
                }

                current = next;
            }
        }

        self.nodes[current].forwards[0]
    }

    fn node_height(&self) -> usize {
        let mut height = 1;

        while height < MAX_HEIGHT && rand::random::<bool>() {
            height += 1;
        }

        height
    }
}

impl Default for MemTable {
    fn default() -> Self {
        Self::new()
    }
}

impl MemTable {
    pub fn new() -> Self {
        Self {
            skiplist: SkipList::new(),
        }
    }

    pub fn put(&mut self, key: Key, value: Value) {
        self.skiplist.insert(key, value);
    }

    pub fn get(&self, key: &[u8]) -> Option<&Value> {
        self.skiplist.search(key)
    }

    pub fn delete(&mut self, key: &[u8]) -> Option<Value> {
        self.skiplist.delete(key)
    }

    pub fn scan(&self, range: KeyRange) -> Vec<KeyValue> {
        self.skiplist.scan(range)
    }

    pub fn len(&self) -> usize {
        self.skiplist.len()
    }

    pub fn is_empty(&self) -> bool {
        self.skiplist.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_returns_inserted_value() {
        let mut list = SkipList::new();

        list.insert(b"bravo".to_vec(), b"2".to_vec());
        list.insert(b"alpha".to_vec(), b"1".to_vec());

        assert_eq!(list.search(b"alpha"), Some(&b"1".to_vec()));
        assert_eq!(list.search(b"bravo"), Some(&b"2".to_vec()));
        assert_eq!(list.search(b"charlie"), None);
    }

    #[test]
    fn insert_replaces_existing_key() {
        let mut list = SkipList::new();

        list.insert(b"alpha".to_vec(), b"old".to_vec());
        list.insert(b"alpha".to_vec(), b"new".to_vec());

        assert_eq!(list.len(), 1);
        assert_eq!(list.search(b"alpha"), Some(&b"new".to_vec()));
    }

    #[test]
    fn delete_unlinks_key() {
        let mut list = SkipList::new();

        list.insert(b"alpha".to_vec(), b"1".to_vec());
        list.insert(b"bravo".to_vec(), b"2".to_vec());
        list.insert(b"charlie".to_vec(), b"3".to_vec());

        assert_eq!(list.delete(b"bravo"), Some(b"2".to_vec()));

        assert_eq!(list.len(), 2);
        assert_eq!(list.search(b"bravo"), None);
        assert_eq!(list.search(b"alpha"), Some(&b"1".to_vec()));
        assert_eq!(list.search(b"charlie"), Some(&b"3".to_vec()));
    }

    #[test]
    fn scan_returns_sorted_exclusive_end_range() {
        let mut table = MemTable::new();

        table.put(b"delta".to_vec(), b"4".to_vec());
        table.put(b"alpha".to_vec(), b"1".to_vec());
        table.put(b"charlie".to_vec(), b"3".to_vec());
        table.put(b"bravo".to_vec(), b"2".to_vec());

        let entries = table.scan(KeyRange::between(b"bravo".to_vec(), b"delta".to_vec()));

        assert_eq!(
            entries,
            vec![
                KeyValue {
                    key: b"bravo".to_vec(),
                    value: b"2".to_vec(),
                },
                KeyValue {
                    key: b"charlie".to_vec(),
                    value: b"3".to_vec(),
                },
            ]
        );
    }
}
