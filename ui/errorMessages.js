const MESSAGES = {
    'missing-credential': "Google API key isn't configured.",
    'unauthorized': 'Google rejected the API key.',
    'network': 'Unable to reach Google Translation.',
    'quota': 'Google Translation quota exceeded.',
    'server': 'Google Translation is temporarily unavailable.',
    'malformed': 'Translation failed.',
};

export const SETTINGS_ACTION_CODES = Object.freeze(['missing-credential']);

export function friendlyMessage(code, fallback = 'Translation failed.') {
    return MESSAGES[code] ?? fallback;
}

export function needsSettingsAction(code) {
    return SETTINGS_ACTION_CODES.includes(code);
}