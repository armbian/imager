// Static option data for the autoconfig profile editor dropdowns.

import type { AutoconfigConfig, UserShell } from '../types';

/** Login shells offered for the first user. */
export const USER_SHELLS: readonly UserShell[] = ['bash', 'zsh'];

/** Convenience shortlist of common UTF-8 locales; the editor also allows free text. */
export const COMMON_LOCALES: readonly string[] = [
  'en_US.UTF-8',
  'en_GB.UTF-8',
  'de_DE.UTF-8',
  'es_ES.UTF-8',
  'fr_FR.UTF-8',
  'it_IT.UTF-8',
  'nl_NL.UTF-8',
  'pl_PL.UTF-8',
  'pt_PT.UTF-8',
  'pt_BR.UTF-8',
  'ru_RU.UTF-8',
  'sv_SE.UTF-8',
  'tr_TR.UTF-8',
  'uk_UA.UTF-8',
  'ja_JP.UTF-8',
  'ko_KR.UTF-8',
  'zh_CN.UTF-8',
  'zh_TW.UTF-8',
  'hr_HR.UTF-8',
  'sl_SI.UTF-8',
  'cs_CZ.UTF-8',
  'fi_FI.UTF-8',
  'da_DK.UTF-8',
  'nb_NO.UTF-8',
  'el_GR.UTF-8',
  'hu_HU.UTF-8',
  'ro_RO.UTF-8',
  'C.UTF-8',
];

/** Returns all IANA timezone names; falls back to a small set on older WebViews lacking Intl.supportedValuesOf. */
export function getTimezones(): string[] {
  const intl = Intl as typeof Intl & {
    supportedValuesOf?: (key: string) => string[];
  };
  if (typeof intl.supportedValuesOf === 'function') {
    try {
      return intl.supportedValuesOf('timeZone');
    } catch {
      // Fall through to the static fallback.
    }
  }
  return ['UTC', 'Europe/London', 'Europe/Berlin', 'America/New_York', 'Asia/Tokyo'];
}

/** Placeholder shown for secret values in the live preview. */
const SECRET_MASK = '••••••••';

/** Mirror of the Rust shell_quote: wrap in double quotes, escape \ " $ `. */
function shellQuote(value: string): string {
  let out = '"';
  for (const ch of value) {
    if (ch === '\\' || ch === '"' || ch === '$' || ch === '`') out += '\\';
    out += ch;
  }
  return out + '"';
}

/** Live preview of the generated /root/.not_logged_in_yet file. */
export interface PresetPreview {
  content: string;
  count: number;
}

/** Display-only preview mirroring the Rust `render_preset` (backend output is authoritative); secrets masked
 * unless `revealSecrets`. */
export function renderPresetPreview(c: AutoconfigConfig, revealSecrets = false): PresetPreview {
  const lines: string[] = [];
  const push = (key: string, value?: string, secret = false) => {
    if (value === undefined || value === '') return;
    const shown = secret && !revealSecrets ? SECRET_MASK : value;
    lines.push(`${key}=${shellQuote(shown)}`);
  };
  const pushBool = (key: string, value?: boolean) => {
    if (value === undefined) return;
    lines.push(`${key}=${value ? '"1"' : '"0"'}`);
  };

  // Network keys are gated on the same flag the backend checks.
  if (c.applyNetwork === true) {
    pushBool('PRESET_NET_CHANGE_DEFAULTS', c.applyNetwork);
    pushBool('PRESET_NET_ETHERNET_ENABLED', c.ethernetEnabled);
    pushBool('PRESET_NET_WIFI_ENABLED', c.wifiEnabled);
    // Wi-Fi credentials only when enabled, so stale SSID/key/country don't linger after toggling off.
    if (c.wifiEnabled) {
      push('PRESET_NET_WIFI_SSID', c.wifiSsid);
      push('PRESET_NET_WIFI_KEY', c.wifiKey, true);
      push('PRESET_NET_WIFI_COUNTRYCODE', c.wifiCountryCode);
    }
    pushBool('PRESET_NET_USE_STATIC', c.useStaticIp);
    // Static address keys only when static IP enabled; otherwise stale values linger after toggling off.
    if (c.useStaticIp) {
      push('PRESET_NET_STATIC_IP', c.staticIp);
      push('PRESET_NET_STATIC_MASK', c.staticMask);
      push('PRESET_NET_STATIC_GATEWAY', c.staticGateway);
      push('PRESET_NET_STATIC_DNS', c.staticDns);
    }
  }

  // Armbian applies locale/timezone only during first-user creation; emit only when a full user is defined.
  const hasUser = !!(c.userName?.trim() && c.userPassword?.trim() && c.userRealName?.trim());
  if (hasUser) {
    push('PRESET_LOCALE', c.locale);
    push('PRESET_TIMEZONE', c.timezone);
    if (c.langBasedOnLocation !== undefined) {
      lines.push(`SET_LANG_BASED_ON_LOCATION=${c.langBasedOnLocation ? '"y"' : '"n"'}`);
    }
  }

  push('PRESET_ROOT_PASSWORD', c.rootPassword, true);
  push('PRESET_ROOT_KEY', c.rootKeyUrl);

  push('PRESET_USER_NAME', c.userName);
  push('PRESET_USER_PASSWORD', c.userPassword, true);
  push('PRESET_USER_KEY', c.userKeyUrl);
  if (c.userShell) lines.push(`PRESET_USER_SHELL=${shellQuote(c.userShell)}`);
  push('PRESET_DEFAULT_REALNAME', c.userRealName);

  push('PRESET_CONFIGURATION', c.remoteConfigUrl);

  return { content: lines.join('\n'), count: lines.length };
}

/** ISO 3166-1 alpha-2 country code paired with an English display name. */
export interface CountryOption {
  code: string;
  name: string;
}

/** ISO 3166-1 alpha-2 country codes for the Wi-Fi regulatory domain selector. */
export const WIFI_COUNTRY_CODES: readonly CountryOption[] = [
  { code: 'AD', name: 'Andorra' },
  { code: 'AE', name: 'United Arab Emirates' },
  { code: 'AF', name: 'Afghanistan' },
  { code: 'AG', name: 'Antigua and Barbuda' },
  { code: 'AI', name: 'Anguilla' },
  { code: 'AL', name: 'Albania' },
  { code: 'AM', name: 'Armenia' },
  { code: 'AO', name: 'Angola' },
  { code: 'AR', name: 'Argentina' },
  { code: 'AT', name: 'Austria' },
  { code: 'AU', name: 'Australia' },
  { code: 'AW', name: 'Aruba' },
  { code: 'AZ', name: 'Azerbaijan' },
  { code: 'BA', name: 'Bosnia and Herzegovina' },
  { code: 'BB', name: 'Barbados' },
  { code: 'BD', name: 'Bangladesh' },
  { code: 'BE', name: 'Belgium' },
  { code: 'BF', name: 'Burkina Faso' },
  { code: 'BG', name: 'Bulgaria' },
  { code: 'BH', name: 'Bahrain' },
  { code: 'BI', name: 'Burundi' },
  { code: 'BJ', name: 'Benin' },
  { code: 'BN', name: 'Brunei Darussalam' },
  { code: 'BO', name: 'Bolivia' },
  { code: 'BR', name: 'Brazil' },
  { code: 'BS', name: 'Bahamas' },
  { code: 'BT', name: 'Bhutan' },
  { code: 'BW', name: 'Botswana' },
  { code: 'BY', name: 'Belarus' },
  { code: 'BZ', name: 'Belize' },
  { code: 'CA', name: 'Canada' },
  { code: 'CD', name: 'Congo (Kinshasa)' },
  { code: 'CF', name: 'Central African Republic' },
  { code: 'CG', name: 'Congo (Brazzaville)' },
  { code: 'CH', name: 'Switzerland' },
  { code: 'CI', name: "Cote d'Ivoire" },
  { code: 'CL', name: 'Chile' },
  { code: 'CM', name: 'Cameroon' },
  { code: 'CN', name: 'China' },
  { code: 'CO', name: 'Colombia' },
  { code: 'CR', name: 'Costa Rica' },
  { code: 'CU', name: 'Cuba' },
  { code: 'CV', name: 'Cabo Verde' },
  { code: 'CY', name: 'Cyprus' },
  { code: 'CZ', name: 'Czechia' },
  { code: 'DE', name: 'Germany' },
  { code: 'DJ', name: 'Djibouti' },
  { code: 'DK', name: 'Denmark' },
  { code: 'DM', name: 'Dominica' },
  { code: 'DO', name: 'Dominican Republic' },
  { code: 'DZ', name: 'Algeria' },
  { code: 'EC', name: 'Ecuador' },
  { code: 'EE', name: 'Estonia' },
  { code: 'EG', name: 'Egypt' },
  { code: 'ER', name: 'Eritrea' },
  { code: 'ES', name: 'Spain' },
  { code: 'ET', name: 'Ethiopia' },
  { code: 'FI', name: 'Finland' },
  { code: 'FJ', name: 'Fiji' },
  { code: 'FM', name: 'Micronesia' },
  { code: 'FR', name: 'France' },
  { code: 'GA', name: 'Gabon' },
  { code: 'GB', name: 'United Kingdom' },
  { code: 'GD', name: 'Grenada' },
  { code: 'GE', name: 'Georgia' },
  { code: 'GH', name: 'Ghana' },
  { code: 'GM', name: 'Gambia' },
  { code: 'GN', name: 'Guinea' },
  { code: 'GQ', name: 'Equatorial Guinea' },
  { code: 'GR', name: 'Greece' },
  { code: 'GT', name: 'Guatemala' },
  { code: 'GW', name: 'Guinea-Bissau' },
  { code: 'GY', name: 'Guyana' },
  { code: 'HN', name: 'Honduras' },
  { code: 'HR', name: 'Croatia' },
  { code: 'HT', name: 'Haiti' },
  { code: 'HU', name: 'Hungary' },
  { code: 'ID', name: 'Indonesia' },
  { code: 'IE', name: 'Ireland' },
  { code: 'IL', name: 'Israel' },
  { code: 'IN', name: 'India' },
  { code: 'IQ', name: 'Iraq' },
  { code: 'IR', name: 'Iran' },
  { code: 'IS', name: 'Iceland' },
  { code: 'IT', name: 'Italy' },
  { code: 'JM', name: 'Jamaica' },
  { code: 'JO', name: 'Jordan' },
  { code: 'JP', name: 'Japan' },
  { code: 'KE', name: 'Kenya' },
  { code: 'KG', name: 'Kyrgyzstan' },
  { code: 'KH', name: 'Cambodia' },
  { code: 'KI', name: 'Kiribati' },
  { code: 'KM', name: 'Comoros' },
  { code: 'KN', name: 'Saint Kitts and Nevis' },
  { code: 'KP', name: 'North Korea' },
  { code: 'KR', name: 'South Korea' },
  { code: 'KW', name: 'Kuwait' },
  { code: 'KZ', name: 'Kazakhstan' },
  { code: 'LA', name: 'Laos' },
  { code: 'LB', name: 'Lebanon' },
  { code: 'LC', name: 'Saint Lucia' },
  { code: 'LI', name: 'Liechtenstein' },
  { code: 'LK', name: 'Sri Lanka' },
  { code: 'LR', name: 'Liberia' },
  { code: 'LS', name: 'Lesotho' },
  { code: 'LT', name: 'Lithuania' },
  { code: 'LU', name: 'Luxembourg' },
  { code: 'LV', name: 'Latvia' },
  { code: 'LY', name: 'Libya' },
  { code: 'MA', name: 'Morocco' },
  { code: 'MC', name: 'Monaco' },
  { code: 'MD', name: 'Moldova' },
  { code: 'ME', name: 'Montenegro' },
  { code: 'MG', name: 'Madagascar' },
  { code: 'MH', name: 'Marshall Islands' },
  { code: 'MK', name: 'North Macedonia' },
  { code: 'ML', name: 'Mali' },
  { code: 'MM', name: 'Myanmar' },
  { code: 'MN', name: 'Mongolia' },
  { code: 'MR', name: 'Mauritania' },
  { code: 'MT', name: 'Malta' },
  { code: 'MU', name: 'Mauritius' },
  { code: 'MV', name: 'Maldives' },
  { code: 'MW', name: 'Malawi' },
  { code: 'MX', name: 'Mexico' },
  { code: 'MY', name: 'Malaysia' },
  { code: 'MZ', name: 'Mozambique' },
  { code: 'NA', name: 'Namibia' },
  { code: 'NE', name: 'Niger' },
  { code: 'NG', name: 'Nigeria' },
  { code: 'NI', name: 'Nicaragua' },
  { code: 'NL', name: 'Netherlands' },
  { code: 'NO', name: 'Norway' },
  { code: 'NP', name: 'Nepal' },
  { code: 'NR', name: 'Nauru' },
  { code: 'NZ', name: 'New Zealand' },
  { code: 'OM', name: 'Oman' },
  { code: 'PA', name: 'Panama' },
  { code: 'PE', name: 'Peru' },
  { code: 'PG', name: 'Papua New Guinea' },
  { code: 'PH', name: 'Philippines' },
  { code: 'PK', name: 'Pakistan' },
  { code: 'PL', name: 'Poland' },
  { code: 'PT', name: 'Portugal' },
  { code: 'PW', name: 'Palau' },
  { code: 'PY', name: 'Paraguay' },
  { code: 'QA', name: 'Qatar' },
  { code: 'RO', name: 'Romania' },
  { code: 'RS', name: 'Serbia' },
  { code: 'RU', name: 'Russia' },
  { code: 'RW', name: 'Rwanda' },
  { code: 'SA', name: 'Saudi Arabia' },
  { code: 'SB', name: 'Solomon Islands' },
  { code: 'SC', name: 'Seychelles' },
  { code: 'SD', name: 'Sudan' },
  { code: 'SE', name: 'Sweden' },
  { code: 'SG', name: 'Singapore' },
  { code: 'SI', name: 'Slovenia' },
  { code: 'SK', name: 'Slovakia' },
  { code: 'SL', name: 'Sierra Leone' },
  { code: 'SM', name: 'San Marino' },
  { code: 'SN', name: 'Senegal' },
  { code: 'SO', name: 'Somalia' },
  { code: 'SR', name: 'Suriname' },
  { code: 'SS', name: 'South Sudan' },
  { code: 'ST', name: 'Sao Tome and Principe' },
  { code: 'SV', name: 'El Salvador' },
  { code: 'SY', name: 'Syria' },
  { code: 'SZ', name: 'Eswatini' },
  { code: 'TD', name: 'Chad' },
  { code: 'TG', name: 'Togo' },
  { code: 'TH', name: 'Thailand' },
  { code: 'TJ', name: 'Tajikistan' },
  { code: 'TL', name: 'Timor-Leste' },
  { code: 'TM', name: 'Turkmenistan' },
  { code: 'TN', name: 'Tunisia' },
  { code: 'TO', name: 'Tonga' },
  { code: 'TR', name: 'Turkiye' },
  { code: 'TT', name: 'Trinidad and Tobago' },
  { code: 'TV', name: 'Tuvalu' },
  { code: 'TW', name: 'Taiwan' },
  { code: 'TZ', name: 'Tanzania' },
  { code: 'UA', name: 'Ukraine' },
  { code: 'UG', name: 'Uganda' },
  { code: 'US', name: 'United States' },
  { code: 'UY', name: 'Uruguay' },
  { code: 'UZ', name: 'Uzbekistan' },
  { code: 'VC', name: 'Saint Vincent and the Grenadines' },
  { code: 'VE', name: 'Venezuela' },
  { code: 'VN', name: 'Vietnam' },
  { code: 'VU', name: 'Vanuatu' },
  { code: 'WS', name: 'Samoa' },
  { code: 'YE', name: 'Yemen' },
  { code: 'ZA', name: 'South Africa' },
  { code: 'ZM', name: 'Zambia' },
  { code: 'ZW', name: 'Zimbabwe' },
];
