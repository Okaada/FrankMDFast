// Canned-response stand-in for @rails/request.js used by golden generators.
// Every call is recorded in `requests`; responses are consumed FIFO from `queue`.
// Queue entries: {ok?, status?, json?} | {reject: "message"} | {abort: true}
export const requests = []
export const queue = []

function serializeBody(body) {
  if (body === undefined || body === null) return body ?? null
  if (typeof FormData !== "undefined" && body instanceof FormData) {
    return { $formData: [...body.entries()].map(([k, v]) => [k, typeof v === "string" ? v : { $file: v.name, size: v.size, type: v.type }]) }
  }
  return body
}

async function request(method, url, options = {}) {
  requests.push({ method, url, responseKind: options.responseKind ?? null, body: serializeBody(options.body), signal: options.signal ? true : undefined })
  const r = queue.shift() ?? { ok: true, status: 200, json: {} }
  if (r.abort) throw Object.assign(new Error("The operation was aborted."), { name: "AbortError" })
  if (r.reject) throw new Error(r.reject)
  return { ok: r.ok ?? true, statusCode: r.status ?? 200, get json() { return Promise.resolve(r.json) }, get text() { return Promise.resolve(JSON.stringify(r.json)) } }
}
export const get = (u, o) => request("GET", u, o)
export const post = (u, o) => request("POST", u, o)
export const patch = (u, o) => request("PATCH", u, o)
export const put = (u, o) => request("PUT", u, o)
export const destroy = (u, o) => request("DELETE", u, o)
