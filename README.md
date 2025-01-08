# Arctis

Arctis is an indexing framework and suite of lightweight tools for fetching, parsing, and transforming Solana blockchain data written in **Rust**. Inspired by Paradigm's [Cryo](https://github.com/paradigmxyz/cryo), it takes a similar modular approach to blockchain data processing but introduces some bigger changes, most notably the use of **DuckDB** as the transformation engine and a design specifically optimized for Solana's much higher throughput.

## Getting Started

### Installation

#### Compile from source

```bash
git clone https://github.com/AlphaArc4k/arctis.git
cd arctis
```
This method requires having rust installed. See rustup for instructions.

**Example 1:** Getting all swaps on pumfun in block 312740977

```bash
cargo run parse block 312740977 --dataset swaps --filter pumpfun
```

Example output
```bash

#############################################
########     AlphaArc Arctis CLI     ########
#############################################

Parse block: 312740977
+----------------------+-----------------------+------------+-----------+-------+---------------+-----------------+-----------+-----------+----------------...
| amount_in            | amount_out            | block_time | dex       | error | signature     | signer          | slot      | swap_type | token          ...
+----------------------+-----------------------+------------+-----------+-------+---------------+-----------------+-----------+-----------+----------------...
| 1.0557094812393188   | 30787216.0            | 1736370445 | "Pumpfun" | false | "3xzBwFwC..." | "BPdVE9EsoD..." | 312740977 | "Buy"     | "BmbRrWLyewsLkX...
+----------------------+-----------------------+------------+-----------+-------+---------------+-----------------+-----------+-----------+----------------...
| 0.5                  | 8592061.0             | 1736370445 | "Pumpfun" | false | "2NE8xoUY..." | "3s7mt8RftK..." | 312740977 | "Buy"     | "Ea7V3B5wAsCMAe...
+----------------------+-----------------------+------------+-----------+-------+---------------+-----------------+-----------+-----------+----------------...
| 0.1001456379890442   | 351248.1875           | 1736370445 | "Pumpfun" | false | "3qoC2bje..." | "PCDq7Rrvd..."  | 312740977 | "Buy"     | "zivexzyFKqt6Ka...
+----------------------+-----------------------+------------+-----------+-------+---------------+-----------------+-----------+-----------+----------------...
| 0.4995000064373016   | 13620355.0            | 1736370445 | "Pumpfun" | false | "w8s6gYn..."  | "2Pxqib8fg...." | 312740977 | "Buy"     | "81kpUi8VsL5uS8...
 ...
 ```

**Example 2:** Parsing all program instructiosn of a single transaction

```bash
cargo run parse tx 5iAwxu7rdRbyUk9N3CtuYdzpK5V864zbSCMvJ7vbGTZaRNBQKZYiK6itBxATdijfitLd2A3ZDYXP1R7GfmrP4fF7
```


## How It Works

1. **RPC Calls with Chunking**  
   Arctis uses Solana RPC endpoints to fetch small block ranges, breaking down large datasets into manageable chunks. This approach minimizes RPC latency issues and prevents timeouts by avoiding long-running requests.

2. **Distributed Processing**  
   Each chunk is assigned to a worker, where the transaction data is decoded and written into a local DuckDB instance. Arctis can handle data transformations directly in-memory, file system or via AWS S3.

   - **DB Engine**  
     Arctis uses DuckDb as in process database to handle data transformations. This enables seamless export into formats like **CSV**, **Parquet**, or other database files while benefiting from a high-performance, columnar query engine. It also allows SQL and Polars processing which can be superior over many lines of custom code e.g. in trading bots to group, filter, and aggregate large amounts of transactions and addresses making it suitable for rapid prototyping and as a library.

   - **Optimized for Solana**  
     Arctis is tailored to Solana's specific data structures and patterns, ensuring better performance and compatibility for Solana-focused workflows.

3. **Recursive Merging**  
   Processed chunks can be recursively merged into larger datasets (5m, 15m, 60m, 12h, 24h..). This step simplifies aggregation and prepares the data for downstream analysis.

4. **Analytics at Scale**
    Arctis can be integrated with tools like Amazon Athena, Trino, and similar federated query engines that enable interactive SQL-like queries directly on data stored in columnar formats such as Parquet. This allows to analyze data at large scale without the need for a dedicated database infrastructure.

## Parsers

Arctis has multiple transaction parsers and decoders and supports the following dexes:
- Jupiter
- Raydium
- Pumpfun


## Performance Considerations

While Arctis is not directly optimized for maximum speed, it is designed to minimize **RPC calls** during decoding and prefers events where possible, significantly reducing the overhead on RPC nodes. There are some optimizations to disable primary keys and have fast batch inserts making it *fast enough* for use in real-time trading tools on modern computers (sub 200ms block parsing).

## Timestamps

Arctis includes helpers and heuristics (binary, linear search) to efficiently work with timestamps instead of block numbers or signatures. This enables time-based chunking and analysis, such as fetching block ranges for specific hours or days. It includes some optimizations for locating transactions within time ranges vs pagination or signature-based searches.

## Scaling with Arctis

Arctis is designed to scale seamlessly in clustered environments:

- **Horizontal Scaling**  
  Add more worker nodes to process additional chunks in parallel. The low-footprint architecture ensures efficient task distribution across large clusters.

- **Vertical Scaling**  
  For setups with better hardware, workers can process multiple chunks simultaneously or multiple processing pipeline steps, maximizing resource usage.

## When Should You Use Arctis?

Arctis is ideal for:

- Teams processing Solana blockchain data at scale.
- Real-time trading tools or monitoring systems (block explorers) are possible but not the primary focus.
- Workflows that benefit from DuckDB’s analytics capabilities and require export formats like Parquet.
- Distributed systems needing a scalable and resource-efficient solution for blockchain data aggregation and analysis.

## Running It Locally

While optimized for clusters, Arctis can be run locally for smaller workloads or real-time monitoring. 
Here are some considerations:

- **Use for Small Workloads**  
  Local execution is best suited for tasks like monitoring real-time activity or handling limited datasets. 

- **RPC Node Usage**  
  Arctis relies on RPC calls, which can place heavy demand on RPC nodes. To avoid overwhelming these nodes:
  - Set local RPC limits in the configuration.
  - Monitor and throttle RPC usage to prevent excessive calls.

## Running It in a Cluster

For large-scale workloads, Arctis can be used on a cluster:

- **Global RPC Limits**  
  Enforce a global RPC usage limit or split load on multiple providers across all nodes to prevent exceeding provider rate limits or causing instability.

- **Task Orchestration**  
  Some task orchestration is needed to distribute tasks evenly across workloads to nodes depending on their spec and recover from errors.



If you have feedback, questions, or contributions, we’d love to hear from you. 
