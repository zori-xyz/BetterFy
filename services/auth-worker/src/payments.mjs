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

export function paymentExpiry(payment, now, plan, currentActiveUntil = now) {
  if (!plan) throw new Error("invalid_payment_plan");
  const reported = Number(payment?.subscription_expiration_date);
  if (plan.recurring && Number.isSafeInteger(reported) && reported > now) return reported;
  return Math.max(now, Number(currentActiveUntil) || now) + plan.durationSeconds;
}

export function isEntitlementActive(row, now) {
  return Boolean(row && Number(row.active_until) > now);
}
