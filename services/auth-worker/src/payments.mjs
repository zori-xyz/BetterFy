export const SUBSCRIPTION_PERIOD_SECONDS = 30 * 24 * 60 * 60;
export const PREMIUM_ENTITLEMENT = "betterfy.premium";

export const ACCESS_PLANS = Object.freeze([
  Object.freeze({ id: "3d", sku: "betterfy-premium-3d", durationSeconds: 3 * 24 * 60 * 60, recurring: false, envKey: "BETTERFY_PLAN_3D_STARS" }),
  Object.freeze({ id: "15d", sku: "betterfy-premium-15d", durationSeconds: 15 * 24 * 60 * 60, recurring: false, envKey: "BETTERFY_PLAN_15D_STARS" }),
  Object.freeze({ id: "30d", sku: "betterfy-premium-30d", durationSeconds: SUBSCRIPTION_PERIOD_SECONDS, recurring: true, envKey: "BETTERFY_PLAN_30D_STARS" }),
]);

export function planById(id) {
  return ACCESS_PLANS.find((plan) => plan.id === id) ?? null;
}

export function planBySku(sku) {
  return ACCESS_PLANS.find((plan) => plan.sku === sku) ?? null;
}

export function priceForPlan(env, plan) {
  return plan ? parseStarsPrice(env?.[plan.envKey]) : null;
}

export function parseStarsPrice(value) {
  const amount = Number(value);
  if (!Number.isSafeInteger(amount) || amount < 1 || amount > 10_000) return null;
  return amount;
}

export function invoicePayload(orderId) {
  if (!/^[0-9a-f-]{36}$/i.test(orderId)) throw new Error("invalid_order_id");
  return `bf_pay:${orderId}`;
}

export function validateCheckout(query, order, telegramUserId) {
  return Boolean(
    query
      && order
      && String(query.from?.id) === String(telegramUserId)
      && query.invoice_payload === order.invoice_payload
      && query.currency === "XTR"
      && order.currency === "XTR"
      && query.total_amount === order.amount
      && ["created", "precheckout", "paid"].includes(order.status),
  );
}

export function validateSuccessfulPayment(payment, order) {
  return Boolean(
    payment
      && order
      && payment.invoice_payload === order.invoice_payload
      && payment.currency === "XTR"
      && order.currency === "XTR"
      && payment.total_amount === order.amount
      && typeof payment.telegram_payment_charge_id === "string"
      && payment.telegram_payment_charge_id.length > 0,
  );
}

// Access is a single queue of paid periods. Each non-refunded charge adds its
// plan duration, starting at its payment time or when the previous period
// ends, whichever is later. Recomputing from the charges (instead of extending
// a stored date) makes a refund remove exactly its own period and lets a
// monthly subscription start after days the user already paid for.
export function entitlementTimeline(charges) {
  const ordered = [...(Array.isArray(charges) ? charges : [])]
    .filter((charge) => planBySku(charge?.sku) && Number.isSafeInteger(Number(charge?.paidAt)))
    .sort((left, right) => Number(left.paidAt) - Number(right.paidAt)
      || String(left.chargeId).localeCompare(String(right.chargeId)));
  let activeUntil = 0;
  let sourceChargeId = null;
  const expiries = new Map();
  for (const charge of ordered) {
    const start = Math.max(Number(charge.paidAt), activeUntil);
    activeUntil = start + planBySku(charge.sku).durationSeconds;
    sourceChargeId = charge.chargeId;
    expiries.set(charge.chargeId, activeUntil);
  }
  return { activeUntil, sourceChargeId, expiries };
}

export function isEntitlementActive(row, now) {
  return Boolean(row && Number(row.active_until) > now);
}
