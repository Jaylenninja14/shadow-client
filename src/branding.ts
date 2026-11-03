// Branding comes from /branding.json so the name, tagline and accent can be changed in one place.
import data from "../branding.json";
import logoUrl from "../branding/logo.svg";

export const branding = {
  productName: data.productName,
  shortName: data.shortName,
  tagline: data.tagline,
  accent: data.accent,
  logoUrl: logoUrl as string,
};
