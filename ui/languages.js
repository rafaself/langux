export const AUTO_LANGUAGE = 'auto';

export const LANGUAGES = [
    {code: 'en', label: 'English'},
    {code: 'pt', label: 'Portuguese'},
    {code: 'es', label: 'Spanish'},
    {code: 'ja', label: 'Japanese'},
    {code: 'fr', label: 'French'},
    {code: 'de', label: 'German'},
    {code: 'it', label: 'Italian'},
    {code: 'ko', label: 'Korean'},
    {code: 'zh', label: 'Chinese'},
];

export function hasLanguage(code) {
    return LANGUAGES.some((language) => language.code === code);
}

export function isExplicit(code) {
    return code !== AUTO_LANGUAGE && hasLanguage(code);
}

export function languageLabel(code) {
    if (code === AUTO_LANGUAGE) return 'Auto detect';
    const language = LANGUAGES.find((l) => l.code === code);
    return language ? language.label : code;
}

export function swapLanguages(source, target) {
    if (!isExplicit(source)) return null;
    return {source: target, target: source};
}
