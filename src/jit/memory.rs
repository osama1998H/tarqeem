//! JIT Memory Management and Optimization
//!
//! Provides memory management utilities for JIT compilation including
//! memory pools, allocation tracking, and optimization hints.
//!
//! إدارة الذاكرة للترجمة الفورية
//! يوفر أدوات لإدارة الذاكرة بما في ذلك
//! تجمعات الذاكرة، تتبع التخصيص، وتلميحات التحسين

use std::collections::HashMap;

/// Memory region type for JIT allocations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryRegion {
    /// Executable code memory
    Code,

    /// Read-only data (constants, strings)
    ReadOnlyData,

    /// Read-write data (globals, heap)
    Data,

    /// Stack memory
    Stack,
}

impl std::fmt::Display for MemoryRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryRegion::Code => write!(f, "Code / كود"),
            MemoryRegion::ReadOnlyData => write!(f, "ReadOnlyData / بيانات للقراءة فقط"),
            MemoryRegion::Data => write!(f, "Data / بيانات"),
            MemoryRegion::Stack => write!(f, "Stack / مكدس"),
        }
    }
}

/// Memory allocation record
#[derive(Debug, Clone)]
pub struct AllocationRecord {
    /// Unique allocation ID
    pub id: u64,

    /// Region where allocated
    pub region: MemoryRegion,

    /// Size in bytes
    pub size: usize,

    /// Allocation purpose/tag
    pub tag: String,

    /// Whether this allocation is still live
    pub is_live: bool,
}

/// Memory usage statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    /// Total bytes allocated per region
    pub allocated: HashMap<MemoryRegion, usize>,

    /// Peak usage per region
    pub peak: HashMap<MemoryRegion, usize>,

    /// Number of allocations per region
    pub allocation_count: HashMap<MemoryRegion, u64>,

    /// Number of deallocations per region
    pub deallocation_count: HashMap<MemoryRegion, u64>,

    /// Current live allocations
    pub live_allocations: u64,
}

impl MemoryStats {
    /// Create new stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Get total allocated memory
    pub fn total_allocated(&self) -> usize {
        self.allocated.values().sum()
    }

    /// Get total peak memory
    pub fn total_peak(&self) -> usize {
        self.peak.values().sum()
    }

    /// Get memory for a specific region
    pub fn region_allocated(&self, region: MemoryRegion) -> usize {
        *self.allocated.get(&region).unwrap_or(&0)
    }

    /// Get peak for a specific region
    pub fn region_peak(&self, region: MemoryRegion) -> usize {
        *self.peak.get(&region).unwrap_or(&0)
    }
}

impl std::fmt::Display for MemoryStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "JIT Memory Statistics / إحصائيات ذاكرة الترجمة الفورية")?;
        writeln!(f, "=======================================================")?;
        writeln!(f)?;

        for region in [
            MemoryRegion::Code,
            MemoryRegion::ReadOnlyData,
            MemoryRegion::Data,
            MemoryRegion::Stack,
        ] {
            let alloc = self.region_allocated(region);
            let peak = self.region_peak(region);
            let count = self.allocation_count.get(&region).unwrap_or(&0);
            let dealloc = self.deallocation_count.get(&region).unwrap_or(&0);

            writeln!(f, "{}:", region)?;
            writeln!(
                f,
                "  Current / الحالي: {} bytes ({:.2} KB)",
                alloc,
                alloc as f64 / 1024.0
            )?;
            writeln!(
                f,
                "  Peak / الذروة: {} bytes ({:.2} KB)",
                peak,
                peak as f64 / 1024.0
            )?;
            writeln!(f, "  Allocations / التخصيصات: {}", count)?;
            writeln!(f, "  Deallocations / الإلغاءات: {}", dealloc)?;
            writeln!(f)?;
        }

        writeln!(
            f,
            "Total / الإجمالي: {} bytes ({:.2} MB)",
            self.total_allocated(),
            self.total_allocated() as f64 / (1024.0 * 1024.0)
        )?;
        writeln!(
            f,
            "Live allocations / التخصيصات الحية: {}",
            self.live_allocations
        )?;

        Ok(())
    }
}

/// Memory tracker for JIT allocations
#[derive(Debug, Default)]
pub struct MemoryTracker {
    /// All allocation records
    allocations: HashMap<u64, AllocationRecord>,

    /// Next allocation ID
    next_id: u64,

    /// Statistics
    stats: MemoryStats,

    /// Whether tracking is enabled
    enabled: bool,
}

impl MemoryTracker {
    /// Create new memory tracker
    pub fn new() -> Self {
        Self {
            allocations: HashMap::new(),
            next_id: 1,
            stats: MemoryStats::new(),
            enabled: false,
        }
    }

    /// Enable tracking
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable tracking
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check if tracking is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Record an allocation
    pub fn record_allocation(
        &mut self,
        region: MemoryRegion,
        size: usize,
        tag: impl Into<String>,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        if self.enabled {
            let record = AllocationRecord {
                id,
                region,
                size,
                tag: tag.into(),
                is_live: true,
            };
            self.allocations.insert(id, record);

            // Update stats
            *self.stats.allocated.entry(region).or_insert(0) += size;
            *self.stats.allocation_count.entry(region).or_insert(0) += 1;
            self.stats.live_allocations += 1;

            // Update peak
            let current = *self.stats.allocated.get(&region).unwrap_or(&0);
            let peak = self.stats.peak.entry(region).or_insert(0);
            if current > *peak {
                *peak = current;
            }
        }

        id
    }

    /// Record a deallocation
    pub fn record_deallocation(&mut self, id: u64) {
        if !self.enabled {
            return;
        }

        if let Some(record) = self.allocations.get_mut(&id) {
            if record.is_live {
                record.is_live = false;

                // Update stats
                *self.stats.allocated.entry(record.region).or_insert(0) =
                    self.stats.allocated[&record.region].saturating_sub(record.size);
                *self
                    .stats
                    .deallocation_count
                    .entry(record.region)
                    .or_insert(0) += 1;
                self.stats.live_allocations = self.stats.live_allocations.saturating_sub(1);
            }
        }
    }

    /// Get current statistics
    pub fn stats(&self) -> &MemoryStats {
        &self.stats
    }

    /// Get all live allocations
    pub fn live_allocations(&self) -> impl Iterator<Item = &AllocationRecord> {
        self.allocations.values().filter(|a| a.is_live)
    }

    /// Get allocations by region
    pub fn allocations_by_region(
        &self,
        region: MemoryRegion,
    ) -> impl Iterator<Item = &AllocationRecord> {
        self.allocations
            .values()
            .filter(move |a| a.region == region && a.is_live)
    }

    /// Clear all records
    pub fn clear(&mut self) {
        self.allocations.clear();
        self.stats = MemoryStats::new();
    }

    /// Generate memory report
    pub fn report(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("{}", self.stats));

        if !self.allocations.is_empty() {
            output.push_str("\nLive Allocations / التخصيصات الحية:\n");
            output.push_str("=====================================\n");

            let mut live: Vec<_> = self.live_allocations().collect();
            live.sort_by(|a, b| b.size.cmp(&a.size));

            for (i, alloc) in live.iter().take(20).enumerate() {
                output.push_str(&format!(
                    "{}. [{}] {} bytes - {}\n",
                    i + 1,
                    alloc.region,
                    alloc.size,
                    alloc.tag
                ));
            }

            if live.len() > 20 {
                output.push_str(&format!("... and {} more\n", live.len() - 20));
            }
        }

        output
    }
}

/// Memory optimization hints
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryHint {
    /// Allocation is short-lived (can use arena)
    ShortLived,

    /// Allocation is long-lived
    LongLived,

    /// Allocation is frequently accessed (keep hot)
    HotPath,

    /// Allocation is rarely accessed (can page out)
    ColdPath,

    /// Allocation size is known at compile time
    FixedSize,

    /// Allocation may grow
    Growable,

    /// Allocation should be aligned
    Aligned(usize),

    /// Allocation is read-only after initialization
    ReadOnly,
}

/// Memory pool for efficient small allocations
#[derive(Debug)]
pub struct MemoryPool {
    /// Pool name
    name: String,

    /// Block size for this pool
    block_size: usize,

    /// Total capacity
    capacity: usize,

    /// Number of blocks allocated
    allocated_blocks: usize,

    /// Free list (indices of free blocks)
    free_list: Vec<usize>,
}

impl MemoryPool {
    /// Create a new memory pool
    pub fn new(name: impl Into<String>, block_size: usize, initial_blocks: usize) -> Self {
        let free_list = (0..initial_blocks).collect();

        Self {
            name: name.into(),
            block_size,
            capacity: initial_blocks,
            allocated_blocks: 0,
            free_list,
        }
    }

    /// Get pool name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get block size
    pub fn block_size(&self) -> usize {
        self.block_size
    }

    /// Get capacity in blocks
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get number of allocated blocks
    pub fn allocated(&self) -> usize {
        self.allocated_blocks
    }

    /// Get number of free blocks
    pub fn available(&self) -> usize {
        self.free_list.len()
    }

    /// Check if pool is full
    pub fn is_full(&self) -> bool {
        self.free_list.is_empty()
    }

    /// Allocate a block, returns block index
    pub fn allocate(&mut self) -> Option<usize> {
        if let Some(index) = self.free_list.pop() {
            self.allocated_blocks += 1;
            Some(index)
        } else {
            None
        }
    }

    /// Free a block (deallocate)
    pub fn deallocate(&mut self, index: usize) {
        if index < self.capacity && !self.free_list.contains(&index) {
            self.free_list.push(index);
            self.allocated_blocks = self.allocated_blocks.saturating_sub(1);
        }
    }

    /// Get utilization percentage
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 {
            0.0
        } else {
            self.allocated_blocks as f64 / self.capacity as f64 * 100.0
        }
    }
}

/// Manager for multiple memory pools
#[derive(Debug, Default)]
pub struct PoolManager {
    /// Named pools
    pools: HashMap<String, MemoryPool>,
}

impl PoolManager {
    /// Create new pool manager
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
        }
    }

    /// Create a new pool
    pub fn create_pool(
        &mut self,
        name: impl Into<String>,
        block_size: usize,
        initial_blocks: usize,
    ) {
        let name = name.into();
        let pool = MemoryPool::new(name.clone(), block_size, initial_blocks);
        self.pools.insert(name, pool);
    }

    /// Get a pool by name
    pub fn get_pool(&self, name: &str) -> Option<&MemoryPool> {
        self.pools.get(name)
    }

    /// Get a mutable pool by name
    pub fn get_pool_mut(&mut self, name: &str) -> Option<&mut MemoryPool> {
        self.pools.get_mut(name)
    }

    /// Remove a pool
    pub fn remove_pool(&mut self, name: &str) {
        self.pools.remove(name);
    }

    /// Get all pool names
    pub fn pool_names(&self) -> impl Iterator<Item = &String> {
        self.pools.keys()
    }

    /// Get total memory used by all pools
    pub fn total_memory(&self) -> usize {
        self.pools.values().map(|p| p.capacity * p.block_size).sum()
    }

    /// Generate pools report
    pub fn report(&self) -> String {
        let mut output = String::new();
        output.push_str("Memory Pools / تجمعات الذاكرة\n");
        output.push_str("============================\n\n");

        for (name, pool) in &self.pools {
            output.push_str(&format!("Pool '{}' / تجمع '{}':\n", name, name));
            output.push_str(&format!(
                "  Block size / حجم الكتلة: {} bytes\n",
                pool.block_size
            ));
            output.push_str(&format!(
                "  Capacity / السعة: {} blocks ({} bytes)\n",
                pool.capacity,
                pool.capacity * pool.block_size
            ));
            output.push_str(&format!(
                "  Allocated / المخصص: {} blocks\n",
                pool.allocated_blocks
            ));
            output.push_str(&format!("  Free / الحر: {} blocks\n", pool.available()));
            output.push_str(&format!(
                "  Utilization / الاستخدام: {:.1}%\n",
                pool.utilization()
            ));
            output.push('\n');
        }

        output.push_str(&format!(
            "Total memory / إجمالي الذاكرة: {} bytes ({:.2} KB)\n",
            self.total_memory(),
            self.total_memory() as f64 / 1024.0
        ));

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_tracker() {
        let mut tracker = MemoryTracker::new();
        tracker.enable();

        let id1 = tracker.record_allocation(MemoryRegion::Code, 1024, "function1");
        let id2 = tracker.record_allocation(MemoryRegion::Code, 2048, "function2");
        let _id3 = tracker.record_allocation(MemoryRegion::Data, 512, "global");

        assert_eq!(tracker.stats().region_allocated(MemoryRegion::Code), 3072);
        assert_eq!(tracker.stats().region_allocated(MemoryRegion::Data), 512);
        assert_eq!(tracker.stats().live_allocations, 3);

        tracker.record_deallocation(id1);
        assert_eq!(tracker.stats().region_allocated(MemoryRegion::Code), 2048);
        assert_eq!(tracker.stats().live_allocations, 2);

        tracker.record_deallocation(id2);
        assert_eq!(tracker.stats().region_allocated(MemoryRegion::Code), 0);
    }

    #[test]
    fn test_memory_pool() {
        let mut pool = MemoryPool::new("test", 64, 10);

        assert_eq!(pool.capacity(), 10);
        assert_eq!(pool.available(), 10);
        assert!(!pool.is_full());

        let idx1 = pool.allocate();
        assert!(idx1.is_some());
        assert_eq!(pool.allocated(), 1);
        assert_eq!(pool.available(), 9);

        let idx2 = pool.allocate();
        assert!(idx2.is_some());
        assert_eq!(pool.allocated(), 2);

        pool.deallocate(idx1.unwrap());
        assert_eq!(pool.allocated(), 1);
        assert_eq!(pool.available(), 9);
    }

    #[test]
    fn test_pool_exhaustion() {
        let mut pool = MemoryPool::new("small", 16, 2);

        assert!(pool.allocate().is_some());
        assert!(pool.allocate().is_some());
        assert!(pool.allocate().is_none()); // Pool exhausted
        assert!(pool.is_full());
    }

    #[test]
    fn test_pool_manager() {
        let mut manager = PoolManager::new();

        manager.create_pool("small", 64, 100);
        manager.create_pool("large", 1024, 10);

        assert!(manager.get_pool("small").is_some());
        assert!(manager.get_pool("large").is_some());
        assert!(manager.get_pool("nonexistent").is_none());

        let total = manager.total_memory();
        assert_eq!(total, 64 * 100 + 1024 * 10);
    }

    #[test]
    fn test_memory_stats() {
        let mut stats = MemoryStats::new();

        stats.allocated.insert(MemoryRegion::Code, 1000);
        stats.allocated.insert(MemoryRegion::Data, 500);
        stats.peak.insert(MemoryRegion::Code, 2000);

        assert_eq!(stats.total_allocated(), 1500);
        assert_eq!(stats.region_allocated(MemoryRegion::Code), 1000);
        assert_eq!(stats.region_peak(MemoryRegion::Code), 2000);
    }

    #[test]
    fn test_pool_utilization() {
        let mut pool = MemoryPool::new("test", 32, 10);

        assert_eq!(pool.utilization(), 0.0);

        pool.allocate();
        pool.allocate();
        pool.allocate();
        pool.allocate();
        pool.allocate(); // 5 out of 10

        assert!((pool.utilization() - 50.0).abs() < 0.1);
    }
}
