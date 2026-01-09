/**
 * Reusable search input component for modal dialogs
 */

import { Search } from 'lucide-react';
import { UI } from '../../config';

interface SearchBoxProps {
  /** Current search value */
  value: string;
  /** Change handler */
  onChange: (value: string) => void;
  /** Placeholder text */
  placeholder: string;
  /** Icon size */
  iconSize?: number;
  /** Auto focus the input */
  autoFocus?: boolean;
}

/**
 * Search input with icon for filtering lists
 */
export function SearchBox({
  value,
  onChange,
  placeholder,
  iconSize = UI.ICON_SIZE.SEARCH,
  autoFocus = true,
}: SearchBoxProps) {
  return (
    <div className="modal-search">
      <div className="search-box" style={{ marginBottom: 0 }}>
        <Search className="search-icon" size={iconSize} />
        <input
          type="text"
          placeholder={placeholder}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className="search-input"
          autoFocus={autoFocus}
        />
      </div>
    </div>
  );
}
