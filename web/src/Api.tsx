import {Event} from './Model';

export default class Api {
    readonly apiBaseUrl: string = window.location.href.substr(0, window.location.href.lastIndexOf(':')) + ':12346';

    getEvents(): Promise<Event[]>  {
        return fetch(this.apiBaseUrl + '/api/v1/event')
            .then(response => response.json())
    }
}