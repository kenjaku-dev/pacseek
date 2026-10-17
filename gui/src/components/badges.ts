// Badges — shared by ResultRow and DetailPane; same semantics as
// `Package::badges()` in the core model.
import type { PackageDto } from "../types";

export function badgeLabels(pkg: PackageDto): string[] {
  const b: string[] = [];
  if (pkg.out_of_date !== null) b.push("OOD");
  if (pkg.orphan) b.push("ORPHAN");
  if (pkg.repo === "aur" && pkg.maintainer === null) b.push("UNMAINTAINED");
  return b;
}
