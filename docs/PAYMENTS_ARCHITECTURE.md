# BetterFy payments and entitlements

This document fixes the payment boundary for BetterFy before any public charge
is enabled. Payments grant an account entitlement; they never unlock Dota or
Steam writes directly.

## Channel decision

BetterFy Premium is a digital service. Purchases shown inside the Telegram bot
or a Telegram Mini App therefore use Telegram Stars (`XTR`) exclusively. The bot
does not show Wallet Pay, cryptocurrency, card, or third-party provider buttons
for the same subscription.

Wallet Pay remains a separate, disabled external-commerce candidate. It may be
evaluated for the public website only after Wallet merchant onboarding, KYC/KYB,
fee and refund review, regional availability review, and confirmation that the
resulting purchase path complies with Telegram and app-store rules. A Wallet API
key must remain server-side. No Wallet order or webhook is accepted today.

## Current plans

| Access | Stars | Billing |
| --- | ---: | --- |
| 3 days | 75 | one-time pass |
| 15 days | 225 | one-time pass |
| 30 days | 525 | recurring every 30 days |

The displayed Stars total is the product price. A fixed USD equivalent is not
promised because the user's purchase price can vary by platform, taxes, and
region.

## Stars purchase flow

1. The user chooses one of the three plans in a private chat with `@BeterFyBot`.
2. The Worker requires a configured integer price between 1 and 10,000 Stars,
   creates a random internal order, and stores the exact currency and amount.
3. Telegram displays a Stars invoice. Only the 30-day plan requests Telegram's
   recurring 30-day subscription period. The invoice payload contains only an
   opaque order ID.
4. The Worker answers `pre_checkout_query` within Telegram's deadline only when
   user, payload, currency, amount, SKU, and order state match the stored order.
5. Access is granted only after a `successful_payment` update. The Telegram
   charge ID is the idempotency key and is retained for support and refunds.
6. Recurring access trusts Telegram's reported expiry. One-time passes extend
   from the later of purchase time or the user's current active expiry.
7. Renewals and additional passes extend the same entitlement. Replayed updates
   do not extend it twice.
8. A refund marks the charge and recalculates access from non-refunded periods.
9. The user can stop renewal; access remains until the paid expiry.

## Stored data

- opaque BetterFy user and order IDs;
- Telegram user ID needed to bind the payment to the account;
- SKU, Stars amount, order state, and timestamps;
- Telegram charge ID, renewal/refund/cancellation facts, and expiry;
- derived entitlement key and active-until timestamp.

The Worker does not store card details, wallet credentials, recovery phrases,
Telegram messages, or a Wallet Pay API key in the repository. Payment records
must have a documented retention and account-deletion policy before release.

## Release gates

- founder-approved Stars price and subscription terms;
- `/paysupport`, privacy terms, refund procedure, and operator response path;
- Cloudflare account, D1 backup policy, production secrets, and redacted logs;
- Telegram test-environment purchase, renewal, cancellation, refund, replay, and
  duplicate-update evidence;
- Windows app verification showing that an expired entitlement removes Premium
  UI access without affecting local builds or recovery data;
- legal and tax review for the operating entity and supported regions.

Until these gates pass, the code is deployable infrastructure, not a publicly
available paid subscription.
