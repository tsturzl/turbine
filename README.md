# Turbine

A flash sympathetic data stream, leveraging latest io_uring features for fast asynchronous IO.


### Requirements

2018 edition Rust
Linux, io_uring is a Linux only feature
Kernel version 5.6+, certain io_uring features used require a relatively modern kernel version

### Goal
Eventually this will be an event stream similar to, and inspired by Apache
Kafka. The ultimate goal is to create a modern event stream borrowing new
concepts from academia, and built to leverage modern hardware and operating
system features.

### Design Decisions

In progress:

  * Leverage io_uring features for network and filesystem access
    * More optimal API, less system calls, less interupts
      * this is especially good in the times where patches for Specter and Meltdown
        reduce system call performance
    * it's easier to share buffers between user and kernel space meaning less copying
    * hypervisor passthru features can improve efficiency during use in many clouds
    * mmap and buffered IO are nice, but we can't control the kernel behavior entirely
        implementing our own virtual memory layer allows us to control our own cache.
  * Safety and reliability are important, anything unsafe needs to be well tested and
    fuzzed.
  * Copying is not fast, don't copy unless it's unsafe not to, opt for safe zero-copy always.
    * This may include avoiding copying memory between user and kernel space.
  * KISS, keep things as simple as possible for desired features
    * Networking should be simple. No need for complex serialization.
      Wire protocol is a simple binary protocol.

Planned:

  * Virtual memory system
    * Leverage O_DIRECT to bypass kernel cacheing
    * Implement a purpose built concurrent paging system
      * Lock free would be great(study the design of sled project)
  * Partitioning
    * Partitions are split by a provided key using hash
  * Replication
    * Partitions are replicated by a given replication factor
  * Membership, consensus, and gossip
    * TBD
    * AP is probably more important than CP in this case, because consistency to the reader
      doesn't mean much since they're mostly waiting for events.
    * consistency only really matters to ensure order from multiple producers, which is still
      not important for most cases.
    * Probably have a raft group per partition
    * Ideas:
      * instead of something like zookeeper perhaps a gossip to share cluster configuration
      * global leaders aren't very scalable, perhaps partition leaders or completely 
        decentralized writes(write to any node of with that partition).
      * clients should be smart
        * clients should send data to nodes who are concerned with that partition, rather than
          having nodes have to forward data they may never store.
          * in a leader system, probably send to the partition leader
          * in a decentralized system any node with that partition will do
        * clients should be informed of the cluster configuration through some mechanism
          so they can make optimal network decisions.
          * either they are part of the gossip "ring"
          * or they periodically grab the configuration from a node
          * access should be read only for clients
    * Be more efficient than Apache Kafka
      * Already don't have a garbage collector
        * Probably avoid using Arc pointers for things that don't live long
        * fewer allocations since we don't need the heap as much
        * the stack is fast
      * io_uring means less system calls, and less copying
      * machine code baby!
