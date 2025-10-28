# tranche_vault

# 🏦 Tranche Vault - Solana Program

A Solana smart contract implementing a two-tranche vault system with Senior and Junior tranches, featuring yield distribution with Senior APY caps and loss waterfalls.

___

## 📖 Overview

This program creates a structured finance vault with two distinct tranches:

🟦 Senior Tranche: Lower risk, capped returns

🟥 Junior Tranche: Higher risk, receives residual yield and absorbs losses first

The vault uses fixed-point arithmetic (1e9 scale) for USD-denominated accounting and operates on a share-based system (like vault tokens).
