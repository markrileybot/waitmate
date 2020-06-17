import * as React from 'react';
import {Event} from './Model';
import {LoadableComponent, LoadableProps, LoadableState} from './LoadableComponent';
import EventCard from "./EventCard";
import {Socket, SocketState} from "./Api";

interface Props extends LoadableProps<Event[]> {
    selected: string,
    onSelect: (id: string) => void,
    messenger: Socket
}

export default class EventList extends LoadableComponent<Event[], Props, LoadableState<Event[]>> {

    state: LoadableState<Event[]> = {
        data: [],
        loading: true,
        error: null,
    }

    onClick(events: Event[]): void {
    }

    componentDidLoad(data: Event[]) {
        let latestData = data[data.length - 1];
        this.props.messenger.addListener(e => {
            data.push(e);
            this.setState({data: data});
        });
        this.props.messenger.addStateListener(s => {
            if (s === SocketState.CONNECTED) {
                this.props.messenger.send("set_offset", {"key": latestData.time + "|" + latestData.id});
            }
        });
    }

    renderError() {
        return (<div>Error: {this.state.error}</div>);
    }

    renderLoaded() {
        if (!this.state.data || this.state.data.length === 0) {
            return (<div>No current events</div>)
        }
        let byCat: {[name: string]: Event[]} = {};
        this.state.data.forEach((e) => {
            let events = byCat[e.category];
            if (!events) {
                byCat[e.category] = events = [];
            }
            events.push(e);
        });
        return (<div className="flex flex-wrap">{Object.values(byCat).map(e => (
            <EventCard key={e[0].category} event={e[e.length-1]} count={e.length}/>
        ))}</div>);
    }

    renderLoading() {
        return (<div>Loading...</div>);
    }
}