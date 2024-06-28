# Allegra: The Integrated Cloud AVS

##### This repository is a WIP

### System Requirements

Allegra currently only works on `Linux x86_64` machines

Allegra has only been tested on the `Ubuntu 22.04 LTS` distro

### Requirements:

- Docker
- tikv (Docker)
- lxd/lxc (snap)
- nginx
- BIND
- BGP


### Description

Allegra is a Decentralized Virtual Private Server network. Users of the network
communicate with the network over gRPC via the CLI or the User Interface (allegra.versatus.io)

Nodes in the network, similarly, communicate over gRPC.

When a node joins the network, it declares itself to one or more `BOOTSTRAP` nodes
the `BOOTSTRAP` nodes then inform the rest of the network about this new peer. 
Peers are stored in a `Distributed Hash Table` (`DHT`), and are allocated to a `Quorum`
using the `AnchorHash` consistent hashring. 

Similarly, VPS instances are allocated to a `Quorum` and are managed via the `DHT`.

Allegra is designed as an asynchronous, eventually consistent, event driven architecture (EDA).

As a result, Allegra is not designed for synchronous consistent, and divergent views of the current state of the network,
whether it be the instances currently available, the quorum responsible, the peers in the network, the current status of tasks, etc. may emerge.

The goal of the network is to consolidate, over time, to a common state.

Dynamic caching is used to ensure that when resources are accessed, either by developers/owners of the instance or users,
the most recently updated instance or resource is available and accessible.

The network is designed, first and foremost, to be fault-tolerant and resilient against attacks.

The network is also designed, at its core, to be decentralized, trustless, permissionless, and censorship resistent.

Instances, and a subset of their state is verifiable
