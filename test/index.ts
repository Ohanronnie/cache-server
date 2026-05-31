import { Cache } from "./utils.ts";

const cache = new Cache({
	host: process.env.CACHE_HOST ?? "127.0.0.1",
	port: Number(process.env.CACHE_PORT ?? "2222"),
	username: process.env.CACHE_USERNAME ?? "admin",
	password: process.env.CACHE_PASSWORD ?? "root",
});

try {
	await cache.set("NAME", "PAUL");
	const name = await cache.get("NAME");
	console.log("value is", name);

	await cache.delete("NAME");
	const deletedName = await cache.get("NAME");
	console.log("deleted value is", deletedName|| "gone");
} finally {
	cache.close();
}
