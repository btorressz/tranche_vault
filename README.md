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
**1️⃣ Two-Tranche Structure**

- Senior Tranche: Protected position with yield capped at a configurable APY

- Junior Tranche: Absorbs first losses, receives all remaining yield above the Senior cap

**2️⃣ Yield Distribution (Waterfall)**
- Total Yield → Senior (up to cap) → Junior (remainder)


- Senior receives the minimum of (yield, cap_amount)

- Junior receives the residual above the cap

- Cap is calculated per period as:
- senior_nav * senior_apy_cap_bps / 10,000

**3️⃣ Loss Distribution (Reverse Waterfall)**
- Total Loss → Junior (first) → Senior (only if Junior depleted)


- Junior absorbs losses up to its NAV

- Senior takes losses only if Junior NAV = 0

**4️⃣ Share-Based Accounting**

- Each tranche issues shares based on Price Per Share (PPS)

- PPS = NAV / shares_supply

- Initial PPS = 1.0 (in fixed-point: 1e9)

  ___


## 🛠️ Program Instructions

### `initialize_vault`

Creates a new vault.

**Parameters:**
- `authority`: Vault manager's pubkey  
- `senior_apy_cap_bps`: Senior APY cap in basis points (e.g., `1000 = 10%`)

---

### `deposit_senior`

Deposit funds into the **Senior** tranche.

**Parameters:**
- `amount_usd_fp`: USD amount in fixed-point (1e9 scale)

**Behavior:**
- Mints shares based on current Senior PPS  
- Updates NAV and total shares  
- Tracks user position via PDA

---

### `deposit_junior`

Deposit funds into the **Junior** tranche.

**Parameters:**
- `amount_usd_fp`: USD amount in fixed-point

**Behavior:**
- Same logic as Senior deposit  
- Junior PPS is calculated independently

---

### `distribute_yield`

Distributes positive yield across both tranches.

**Parameters:**
- `yield_fp`: Total yield in fixed-point

**Behavior:**
- Calculates Senior yield cap  
- Allocates yield using waterfall logic  
- Emits `YieldDistributed` event

---

### `simulate_loss` *(Authority only)*

Simulates portfolio losses for testing or stress analysis.

**Parameters:**
- `total_loss_fp`: Loss amount in fixed-point

**Behavior:**
- Junior absorbs first  
- Senior absorbs remainder if Junior depleted  
- Updates NAVs accordingly

---

### `simulate_yield_surplus` *(Authority only)*

Simulates yield generation for testing and dry runs.

**Parameters:**
- `amount_usd_fp`: Simulated yield in fixed-point

**Behavior:**
- Follows same logic as `distribute_yield`  
- Emits `SimulatedYield` event

---
