// Six cells for a six-digit code, as on the Telegram code screen. One real
// input sits over the cells, so paste, the system's one-time-code autofill and
// screen readers keep working; the cells only draw what it holds. The gap in
// the middle matches how emails and the bot write the code ("022 170").
export function CodeCells({
  id,
  value,
  onChange,
  error = false,
  autoFocus = false,
  describedBy,
}: {
  id: string;
  value: string;
  onChange: (code: string) => void;
  error?: boolean;
  autoFocus?: boolean;
  describedBy?: string;
}) {
  return (
    <div
      className={`otp-field code-cells${error ? " is-error" : ""}${value.length === 6 ? " is-complete" : ""}`}
    >
      <input
        id={id}
        autoFocus={autoFocus}
        autoComplete="one-time-code"
        inputMode="numeric"
        pattern="[0-9]*"
        maxLength={6}
        value={value}
        onChange={(event) => onChange(event.target.value.replace(/\D/g, "").slice(0, 6))}
        aria-invalid={error}
        aria-describedby={describedBy}
      />
      <div className="otp-cells" aria-hidden="true">
        {Array.from({ length: 6 }, (_, index) => (
          <i
            className={value[index] ? "filled" : index === value.length ? "current" : ""}
            key={index}
          >
            {value[index] ?? ""}
          </i>
        ))}
      </div>
    </div>
  );
}
