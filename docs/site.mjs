export const site = 'https://gehirudm.github.io';
export const base = '/net_lang';
export const repository = 'https://github.com/gehirudm/net_lang';
export const pageUrl = (slug) => `${site}${base}/${slug === 'index' ? '' : `${slug}/`}`;
