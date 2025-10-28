# tranche_vault

# 🏦 Tranche Vault - Solana Program

A Solana smart contract implementing a two-tranche vault system with Senior and Junior tranches, featuring yield distribution with Senior APY caps and loss waterfalls.

___

## 📖 Overview

This program creates a structured finance vault with two distinct tranches:

🟦 Senior Tranche: Lower risk, capped returns

🟥 Junior Tranche: Higher risk, receives residual yield and absorbs losses first

The vault uses fixed-point arithmetic (1e9 scale) for USD-denominated accounting and operates on a share-based system (like vault tokens).

___

## ✨ Key Features
1️⃣ Two-Tranche Structure

Senior Tranche: Protected position with yield capped at a configurable APY

Junior Tranche: Absorbs first losses, receives all remaining yield above the Senior cap

2️⃣ Yield Distribution (Waterfall)
Total Yield → Senior (up to cap) → Junior (remainder)


Senior receives the minimum of (yield, cap_amount)

Junior receives the residual above the cap

Cap is calculated per period as:
senior_nav * senior_apy_cap_bps / 10,000

3️⃣ Loss Distribution (Reverse Waterfall)
Total Loss → Junior (first) → Senior (only if Junior depleted)


Junior absorbs losses up to its NAV

Senior takes losses only if Junior NAV = 0

4️⃣ Share-Based Accounting

Each tranche issues shares based on Price Per Share (PPS)

PPS = NAV / shares_supply

Initial PPS = 1.0 (in fixed-point: 1e9)
