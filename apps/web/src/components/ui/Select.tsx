import type { SelectHTMLAttributes } from 'react';

export type Option = { value: string; label: string };

/** Native select: best mobile behaviour and keyboard support, styled by the global form rules. */
export function Select({ options, placeholder, ...rest }: SelectHTMLAttributes<HTMLSelectElement> & { options: Option[]; placeholder?: string }) {
  return (
    <select {...rest}>
      {placeholder !== undefined && <option value="">{placeholder}</option>}
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}
