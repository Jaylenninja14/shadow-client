import { createContext, useContext, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { AlertTriangle, CheckCircle2, Info, Package, X, XCircle } from "lucide-react";

export function Button(props: {
  children?: ReactNode;
  onClick?: () => unknown | Promise<unknown>;
  variant?: "primary" | "danger" | "ghost" | "default";
  size?: "sm" | "md";
  disabled?: boolean;
  title?: string;
  className?: string;
  icon?: ReactNode;
}) {
  const [busy, setBusy] = useState(false);
  const cls = ["btn", props.variant && props.variant !== "default" ? props.variant : "", props.size === "sm" ? "sm" : "", props.className ?? ""].join(" ");
  return (
    <button
      className={cls}
      disabled={props.disabled || busy}
      title={props.title}
      aria-label={props.children ? undefined : props.title}
      onClick={async () => {
        if (!props.onClick) return;
        const r = props.onClick();
        if (r instanceof Promise) {
          setBusy(true);
          try {
            await r;
          } finally {
            setBusy(false);
          }
        }
      }}
    >
      {busy ? <span className="spinner" /> : props.icon}
      {props.children}
    </button>
  );
}

export function Toggle({ on, onChange, disabled, label }: { on: boolean; onChange: (v: boolean) => void; disabled?: boolean; label?: string }) {
  return <button role="switch" aria-checked={on} aria-label={label} className={`toggle ${on ? "on" : ""}`} disabled={disabled} onClick={() => onChange(!on)} />;
}

/** Text typed into a settings search; fields that don't mention it hide themselves. */
export const FieldSearch = createContext("");

export function Field({ label, hint, children, full }: { label: string; hint?: ReactNode; children: ReactNode; full?: boolean }) {
  const q = useContext(FieldSearch);
  if (q && !label.toLowerCase().includes(q) && !(typeof hint === "string" && hint.toLowerCase().includes(q))) return null;
  return (
    <div className={`field ${full ? "full" : ""}`}>
      <label>{label}</label>
      {children}
      {hint && <div className="hint">{hint}</div>}
    </div>
  );
}

export function Badge({ children, tone }: { children: ReactNode; tone?: "accent" | "danger" | "warning" | "info" }) {
  return <span className={`badge ${tone ?? ""}`}>{children}</span>;
}

export function Progress({ value, indeterminate }: { value: number; indeterminate?: boolean }) {
  return (
    <div className={`progress ${indeterminate ? "indeterminate" : ""}`}>
      <div style={{ width: `${Math.max(0, Math.min(100, value))}%` }} />
    </div>
  );
}

export function Spinner() {
  return <span className="spinner" />;
}

export function Tabs<T extends string>({ tabs, value, onChange }: { tabs: { id: T; label: ReactNode }[]; value: T; onChange: (v: T) => void }) {
  // a highlight that slides to the chosen tab
  const ref = useRef<HTMLDivElement>(null);
  const pill = useSlidingPill(ref, ".tab.active", [value, tabs.length]);
  return (
    <div className="tabs" role="tablist" ref={ref}>
      {pill && <span className="tab-pill" aria-hidden style={pill} />}
      {tabs.map((t) => (
        <button key={t.id} role="tab" aria-selected={value === t.id} className={`tab ${value === t.id ? "active" : ""}`} onClick={() => onChange(t.id)}>
          {t.label}
        </button>
      ))}
    </div>
  );
}

/** Where the element matching `selector` sits inside `box`, as a style for a highlight that slides there. */
export function useSlidingPill(box: React.RefObject<HTMLElement | null>, selector: string, deps: unknown[]): React.CSSProperties | null {
  const [pill, setPill] = useState<React.CSSProperties | null>(null);
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const place = () => {
      const on = el.querySelector<HTMLElement>(selector);
      setPill(on ? { transform: `translate(${on.offsetLeft}px, ${on.offsetTop}px)`, width: on.offsetWidth, height: on.offsetHeight } : null);
    };
    place();
    const ro = new ResizeObserver(place);
    ro.observe(el);
    return () => ro.disconnect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  return pill;
}

export function Empty({ title, children, icon }: { title: string; children?: ReactNode; icon?: ReactNode }) {
  return (
    <div className="empty">
      {icon ?? <Package size={36} />}
      <h3>{title}</h3>
      {children && <div className="mt">{children}</div>}
    </div>
  );
}

export function Alert({ tone, title, children, action }: { tone: "error" | "warning" | "info" | "success"; title?: ReactNode; children?: ReactNode; action?: ReactNode }) {
  const Icon = tone === "error" ? XCircle : tone === "warning" ? AlertTriangle : tone === "success" ? CheckCircle2 : Info;
  const color = tone === "error" ? "var(--danger)" : tone === "warning" ? "var(--warning)" : tone === "success" ? "var(--success)" : "var(--info)";
  return (
    <div className={`alert ${tone}`}>
      <Icon size={17} color={color} />
      <div className="grow">
        {title && <div className="title">{title}</div>}
        {children && <div className="dim" style={{ whiteSpace: "pre-wrap" }}>{children}</div>}
      </div>
      {action}
    </div>
  );
}

export function Modal({ title, onClose, children, footer, wide }: { title: ReactNode; onClose: () => void; children: ReactNode; footer?: ReactNode; wide?: boolean }) {
  return (
    <div className="modal-bg" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className={`modal ${wide ? "wide" : ""}`} role="dialog" aria-modal="true">
        <div className="modal-head">
          <h2 className="grow">{title}</h2>
          <button className="btn ghost icon" onClick={onClose} aria-label="Close">
            <X size={18} />
          </button>
        </div>
        <div className="modal-body">{children}</div>
        {footer && <div className="modal-foot">{footer}</div>}
      </div>
    </div>
  );
}

/** Confirmation dialog for destructive or significant actions. */
export function Confirm({ title, children, confirmLabel, danger, onConfirm, onCancel }: { title: string; children: ReactNode; confirmLabel: string; danger?: boolean; onConfirm: () => Promise<unknown> | unknown; onCancel: () => void }) {
  return (
    <Modal
      title={title}
      onClose={onCancel}
      footer={
        <>
          <Button onClick={onCancel}>Cancel</Button>
          <Button variant={danger ? "danger" : "primary"} onClick={async () => { await onConfirm(); }}>
            {confirmLabel}
          </Button>
        </>
      }
    >
      {children}
    </Modal>
  );
}

export function Img({ src, size = 40, alt = "" }: { src: string | null | undefined; size?: number; alt?: string }) {
  const [broken, setBroken] = useState(false);
  return (
    <div className="icon-box" style={{ width: size, height: size }}>
      {src && !broken ? <img src={src} alt={alt} loading="lazy" onError={() => setBroken(true)} /> : <Package size={size * 0.5} />}
    </div>
  );
}

export function Select<T extends string>({ value, onChange, options, disabled, style }: { value: T; onChange: (v: T) => void; options: { value: T; label: string }[]; disabled?: boolean; style?: React.CSSProperties }) {
  return (
    <select className="select" value={value} disabled={disabled} onChange={(e) => onChange(e.target.value as T)} style={style}>
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}
