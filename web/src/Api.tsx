import {Event} from './Model';

export enum SocketState {
    CLOSED, CONNECTING, CONNECTED, FAILED
}

export class Socket {
    private listeners: ((e: Event) => void)[] = [];
    private stateListeners: ((s: SocketState) => void)[] = [];
    private ws: WebSocket | null;
    private handle: any;
    private state: SocketState;
    private readonly url: string;

    constructor(url: string) {
        this.url = url;
        this.ws = null;
        this.state = SocketState.CLOSED;
        this.connect();
    }

    addListener(l: (e: Event) => void): void {
        this.listeners.push(l);
    }

    addStateListener(l: (s: SocketState) => void): void {
        this.stateListeners.push(l);
        l(this.state);
    }

    send(cmd: string, args: Object) {
        if (this.ws) {
            this.ws.send(JSON.stringify({
                'command': cmd,
                'args': args
            }));
        }
    }

    private connect() {
        this.ws = new WebSocket(this.url);
        this.ws.onmessage = this.onMessage.bind(this);
        this.ws.onopen = this.onOpen.bind(this);
        this.ws.onclose = this.onClose.bind(this);
        this.ws.onerror = this.onError.bind(this);
    }

    private onOpen() {
        this.handle = window.setInterval(() => {
            this.send("ping", {});
        }, 3000);
        this.setState(SocketState.CONNECTED);
    }

    private onClose() {
        this.setState(SocketState.CLOSED);
        if (this.handle) {
            window.clearTimeout(this.handle);
            window.clearInterval(this.handle);
            this.handle = null;
        }
        this.setState(SocketState.CONNECTING);
        this.handle = window.setTimeout(() => {
            this.connect();
        }, 3000);
    }

    private onError() {
        this.setState(SocketState.FAILED);
    }

    private onMessage(evt: MessageEvent) {
        const data: Event = JSON.parse(evt.data);
        this.listeners.forEach((l) => {l(data);});
    }

    private setState(s: SocketState) {
        this.state = s;
        this.stateListeners.forEach((l) => {l(s);});
    }
}

export default class Api {
    readonly apiBaseUrl: string = (function() {
        if (!process.env.NODE_ENV || process.env.NODE_ENV === 'development') {
            return window.location.href.substr(0, window.location.href.lastIndexOf(':')) + ':12346/';
        } else {
            return window.location.href;
        }
    })();

    getEvents(): Promise<Event[]>  {
        return fetch(this.apiBaseUrl + 'api/v1/event')
            .then(response => response.json())
    }

    getSocket(): Socket {
        return new Socket(this.apiBaseUrl.replace("http", "ws") + "api/v1/connect");
    }
}