import type { ReactNode } from "react";

/** Line icons (lucide geometry, ISC licence): one stroke, colour from the surrounding text role. */
function Icon({ size = 14, className, children }: { size?: number; className?: string; children: ReactNode }) {
  return <svg className={className ? `icon ${className}` : "icon"} width={size} height={size} viewBox="0 0 24 24"
    fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    {children}
  </svg>;
}

interface IconProps { size?: number; className?: string }

export const EyeIcon = (props: IconProps) => <Icon {...props}>
  <path d="M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0" />
  <circle cx="12" cy="12" r="3" />
</Icon>;

export const EyeOffIcon = (props: IconProps) => <Icon {...props}>
  <path d="M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49" />
  <path d="M14.084 14.158a3 3 0 0 1-4.242-4.242" />
  <path d="M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143" />
  <path d="m2 2 20 20" />
</Icon>;

export const ErrorIcon = (props: IconProps) => <Icon {...props}>
  <circle cx="12" cy="12" r="10" /><path d="m15 9-6 6" /><path d="m9 9 6 6" />
</Icon>;

export const WarningIcon = (props: IconProps) => <Icon {...props}>
  <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3" />
  <path d="M12 9v4" /><path d="M12 17h.01" />
</Icon>;

export const InfoIcon = (props: IconProps) => <Icon {...props}>
  <circle cx="12" cy="12" r="10" /><path d="M12 16v-4" /><path d="M12 8h.01" />
</Icon>;

export const CheckIcon = (props: IconProps) => <Icon {...props}>
  <circle cx="12" cy="12" r="10" /><path d="m9 12 2 2 4-4" />
</Icon>;

export const CloseIcon = (props: IconProps) => <Icon {...props}>
  <path d="M18 6 6 18" /><path d="m6 6 12 12" />
</Icon>;

export const SaveIcon = (props: IconProps) => <Icon {...props}>
  <path d="M15.2 3a2 2 0 0 1 1.4.6l3.8 3.8a2 2 0 0 1 .6 1.4V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z" />
  <path d="M17 21v-7a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v7" /><path d="M7 3v4a1 1 0 0 0 1 1h7" />
</Icon>;

export const ProjectIcon = (props: IconProps) => <Icon {...props}>
  <path d="m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2" />
</Icon>;

/** Severity marker for diagnostics: icon plus sentence-case word, coloured by the parent. */
export function SeverityIcon({ severity }: { severity: "info" | "warning" | "error" }) {
  return severity === "error" ? <ErrorIcon size={12} /> : severity === "warning" ? <WarningIcon size={12} /> : <InfoIcon size={12} />;
}
