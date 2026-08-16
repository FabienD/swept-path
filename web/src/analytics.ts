/**
 * Vercel Web Analytics.
 *
 * Counts page views on the deployed site. It is cookie-free and stores
 * nothing on the visitor's machine, which is what makes it acceptable in a
 * tool that otherwise sends nothing anywhere: the whole simulation runs in
 * the browser, and no measurement anyone types ever leaves it.
 *
 * That property is worth keeping true rather than merely claiming, so
 * `beforeSend` strips query strings before an event goes out. Nothing puts a
 * measurement in the URL today — but a share-this-scene link is an obvious
 * thing to add later, and it would start posting people's gateway dimensions
 * to an analytics endpoint without anyone noticing.
 *
 * Development is excluded outright: local runs are not visits.
 */
import { inject } from "@vercel/analytics";

export function startAnalytics(): void {
  if (import.meta.env.DEV) return;

  inject({
    mode: "production",
    beforeSend: (event) => ({
      ...event,
      url: event.url.split("?")[0] ?? event.url,
    }),
  });
}
