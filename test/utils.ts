import net from "net";

const Step = {
	ResponseTypeLength: 0,
	ResponseType: 1,
	ValueLength: 2,
	Value: 3,
} as const;

type Step = (typeof Step)[keyof typeof Step];

type CacheOptions = {
	host?: string;
	port: number;
	username: string;
	password: string;
	timeoutMs?: number;
};

type PendingRequest = {
	id: number;
	command: string;
	key: string;
	resolve: (value: string) => void;
	reject: (error: Error) => void;
};

type ParsedResponse = {
	type: string;
	value: string;
};

export class Cache {
	private readonly socket: net.Socket;
	private readonly timeoutMs: number;
	private readonly pending = new Map<number, PendingRequest>();
	private readonly pendingOrder: number[] = [];
	private readonly buffer: number[] = [];
	private nextRequestId = 1;
	private step: Step = Step.ResponseTypeLength;
	private length = 0;
	private responseType = "";
	private ready: Promise<void>;

	constructor(options: CacheOptions) {
		this.timeoutMs = options.timeoutMs ?? 5_000;
		this.socket = net.createConnection({
			host: options.host ?? "127.0.0.1",
			port: options.port,
		});

		this.socket.on("data", (data) => this.onData(data));
		this.socket.on("error", (error) => this.rejectAll(error));
		this.socket.on("close", () => this.rejectAll(new Error("Connection closed")));

		this.ready = new Promise<void>((resolve, reject) => {
			const timeout = setTimeout(() => {
				reject(new Error("Connection timed out"));
				this.socket.destroy();
			}, this.timeoutMs);

			this.socket.once("connect", () => {
				clearTimeout(timeout);
				resolve();
			});

			this.socket.once("error", (error) => {
				clearTimeout(timeout);
				reject(error);
			});
		}).then(() => this.auth(options.username, options.password));
	}

	async get(key: string): Promise<string> {
		await this.ready;
		return this.send("GET", key);
	}

	async set(key: string, value: string): Promise<void> {
		await this.ready;
		await this.send("SET", key, value);
	}

	async delete(key: string): Promise<void> {
		await this.ready;
		await this.send("DELETE", key);
	}

	close() {
		this.socket.end();
	}

	private async auth(username: string, password: string): Promise<void> {
		const response = await this.send("AUTH", username, password);
		if (response !== "OK") {
			throw new Error("Authentication failed");
		}
	}

	private send(command: string, key: string, value?: string): Promise<string> {
		const id = this.nextRequestId++;
		const payload = encodeRequest(command, key, value);

		return new Promise((resolve, reject) => {
			const timeout = setTimeout(() => {
				this.pending.delete(id);
				const index = this.pendingOrder.indexOf(id);
				if (index !== -1) {
					this.pendingOrder.splice(index, 1);
				}
				reject(new Error(`Request timed out: ${command} ${key}`));
			}, this.timeoutMs);

			this.pending.set(id, {
				id,
				command,
				key,
				resolve: (value) => {
					clearTimeout(timeout);
					resolve(value);
				},
				reject: (error) => {
					clearTimeout(timeout);
					reject(error);
				},
			});
			this.pendingOrder.push(id);
			this.socket.write(payload);
		});
	}

	private onData(data: Buffer | string) {
		if (typeof data === "string") {
			this.buffer.push(...new TextEncoder().encode(data));
		} else {
			this.buffer.push(...data);
		}

		for (const response of this.parseResponses()) {
			this.resolveNext(response);
		}
	}

	private parseResponses(): ParsedResponse[] {
		const responses: ParsedResponse[] = [];

		while (true) {
			switch (this.step) {
				case Step.ResponseTypeLength: {
					const value = parseLength(this.buffer);
					if (value === undefined) return responses;
					this.length = value;
					this.step = Step.ResponseType;
					break;
				}
				case Step.ResponseType: {
					const value = parseField(this.buffer, this.length);
					if (value === undefined) return responses;
					this.responseType = value;
					this.step = Step.ValueLength;
					break;
				}
				case Step.ValueLength: {
					const value = parseLength(this.buffer);
					if (value === undefined) return responses;
					this.length = value;
					this.step = Step.Value;
					break;
				}
				case Step.Value: {
					const value = parseField(this.buffer, this.length);
					if (value === undefined) return responses;
					responses.push({ type: this.responseType, value });
					this.responseType = "";
					this.step = Step.ResponseTypeLength;
					break;
				}
			}
		}
	}

	private resolveNext(response: ParsedResponse) {
		const requestId = this.pendingOrder.shift();
		if (requestId === undefined) return;

		const request = this.pending.get(requestId);
		if (!request) return;

		this.pending.delete(requestId);
		if (response.type === "ERROR") {
			request.reject(new Error(response.value));
		} else {
			request.resolve(response.value);
		}
	}

	private rejectAll(error: Error) {
		for (const request of this.pending.values()) {
			request.reject(error);
		}
		this.pending.clear();
		this.pendingOrder.length = 0;
	}
}

export function convertToString(buffer: number[]): string {
	return new TextDecoder().decode(new Uint8Array(buffer));
}

export function parseLength(buffer: number[]) {
	const position = findDollar(buffer);
	if (position === -1) return;

	const lengthBuf = buffer.slice(0, position);
	const lengthString = convertToString(lengthBuf);
	const length = Number.parseInt(lengthString, 10);

	if (Number.isNaN(length)) throw new TypeError("Contains invalid values");
	buffer.splice(0, position + 1);

	return length;
}

export function parseField(buffer: number[], length: number) {
	if (buffer.length < length + 1) return;

	const data = buffer.splice(0, length + 1);
	if (data[length] !== "$".charCodeAt(0)) {
		throw new TypeError("Invalid command");
	}

	data.pop();
	return convertToString(data);
}

function encodeRequest(command: string, key: string, value?: string): string {
	const parts = [field(command.toUpperCase()), field(key)];
	if (value !== undefined) {
		parts.push(field(value));
	}

	return parts.join("");
}

function field(value: string): string {
	return `${new TextEncoder().encode(value).length}$${value}$`;
}

function findDollar(buffer: number[]): number {
	return buffer.findIndex((value) => value === "$".charCodeAt(0));
}
