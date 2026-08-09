import test from "node:test";
import assert from "node:assert/strict";
import {
  ACCESS_PLANS,
  SUBSCRIPTION_PERIOD_SECONDS,
  invoicePayload,
  isEntitlementActive,
  parseStarsPrice,
  paymentExpiry,
  planById,
  planBySku,
  priceForPlan,
  validateCheckout,
  validateSuccessfulPayment,
} from "../src/payments.mjs";

const order = {
  invoice_payload: "bf_pay:11111111-1111-4111-8111-111111111111",
  currency: "XTR",
  amount: 250,
  status: "created",
};

test("Stars price must be an integer within Telegram subscription limits", () => {
  assert.equal(parseStarsPrice("250"), 250);
  assert.equal(parseStarsPrice("0"), null);
  assert.equal(parseStarsPrice("10001"), null);
  assert.equal(parseStarsPrice("12.5"), null);
  assert.equal(parseStarsPrice("not-set"), null);
});

test("access plans keep short passes one-time and monthly access recurring", () => {
  assert.equal(ACCESS_PLANS.length, 3);
  assert.equal(planById("3d").recurring, false);
  assert.equal(planById("15d").durationSeconds, 15 * 24 * 60 * 60);
  assert.equal(planBySku("betterfy-premium-30d").recurring, true);
  assert.equal(priceForPlan({ BETTERFY_PLAN_3D_STARS: "75" }, planById("3d")), 75);
  assert.equal(priceForPlan({}, planById("3d")), null);
});

test("invoice payload is bounded to a valid opaque order id", () => {
  assert.equal(
    invoicePayload("11111111-1111-4111-8111-111111111111"),
    order.invoice_payload,
  );
  assert.throws(() => invoicePayload("user-controlled-payload"), /invalid_order_id/);
});

test("pre-checkout requires matching user, payload, currency, amount, and state", () => {
  const query = {
    from: { id: 42 },
    invoice_payload: order.invoice_payload,
    currency: "XTR",
    total_amount: 250,
  };
  assert.equal(validateCheckout(query, order, "42"), true);
  assert.equal(validateCheckout({ ...query, total_amount: 251 }, order, "42"), false);
  assert.equal(validateCheckout(query, { ...order, status: "refunded" }, "42"), false);
  assert.equal(validateCheckout(query, order, "43"), false);
});

test("successful payment must still match the stored order", () => {
  const payment = {
    invoice_payload: order.invoice_payload,
    currency: "XTR",
    total_amount: 250,
    telegram_payment_charge_id: "charge-1",
  };
  assert.equal(validateSuccessfulPayment(payment, order), true);
  assert.equal(validateSuccessfulPayment({ ...payment, currency: "USD" }, order), false);
  assert.equal(validateSuccessfulPayment({ ...payment, telegram_payment_charge_id: "" }, order), false);
});

test("Telegram expiry wins for monthly access and one-time passes extend safely", () => {
  assert.equal(paymentExpiry({ subscription_expiration_date: 5000 }, 1000, planById("30d")), 5000);
  assert.equal(paymentExpiry({}, 1000, planById("30d")), 1000 + SUBSCRIPTION_PERIOD_SECONDS);
  assert.equal(paymentExpiry({}, 1000, planById("3d"), 2000), 2000 + 3 * 24 * 60 * 60);
  assert.equal(isEntitlementActive({ active_until: 1001 }, 1000), true);
  assert.equal(isEntitlementActive({ active_until: 1000 }, 1000), false);
});
