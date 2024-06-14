# Transaction Processor
This repo contains a toy implementation of a payment processing system. From an input CSV containing rows of transactions, the main program binary will generate a new CSV containing the resulting account states derived from the processed transactions.

This program currently supports Withdrawal, Desposit, Dispute, Chargeback, and Resolution actions from an input stream of chronologically ordered transactions. Further work will be required to properly handled multiple input streams and unordered transactions.

This repo is divided into two main components:
- Domain
- Engine

To run this program, make sure to install Rust and clone this repo locally. From the root folder: `cargo run -- <path_to_csv>`. I have included an example CSV which can be processed with `cargo run -- transaction.csv`.

## Domain
This module contains the Type definitions for Accounts, Transactions, Error variants, and Transaction History. These Types can be modified indpendently from the Engine to allow for iterative improvements or handling new use cases.

## Engine
This module contains the driving logic for the app: a state machine trait definition and implementation that currently handles synchronous inputs but could also be adapted for other use cases in the future.

Unit tests for expected interactions between transactions and accounts can be found in this Module.

## Assumptions
#### Dispute
Disputing a withdrawal should have a different effect than disputing a deposit. Concretely: disputing a withdrawal should increase the account total and held total, wherease disputing a deposit should increase the held total and decrease the available total.

#### Resolve
If a dispute is resolved, the funds should clear in favor of the account owner. Concretely: if an account has 100 units being held at the time of resolution, those units should be added to the account's available balance.

#### Chargeback
If a dispute results in a chargeback, the held funds are removed from the account's held and total trackers, then the user's account becomes locked. Unlocking the accounts is not currently supported, so no transactions will be valid against this account until the program is finished.


## Future Work
  - Add support for multiple input streams
  - Handling for unordered transactions
  - Ability to read and write transaction history from persisted source, not RAM or HEAP.
  - Machine implementation that handles concurrent Hashmap access