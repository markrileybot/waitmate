import * as React from 'react';
import {Event} from './Model';
import {LoadableComponent, LoadableProps, LoadableState} from './LoadableComponent';

interface Props extends LoadableProps<Event[]> {
    selected: string,
    onSelect: (id: string) => void
}

export default class EventList extends LoadableComponent<Event[], Props, LoadableState<Event[]>> {

    state: LoadableState<Event[]> = {
        data: [],
        loading: true,
        error: null,
    }

    onClick(id: string): void {
        if (id === "1") {
            alert("It says there isn't any!");
        }
        this.props.onSelect(id);
    }

    componentDidLoad(data: Event[]) {
        data.sort((g0, g1) => {
            return -((g0.time || 0) - (g1.time || 0));
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

        Object.entries(byCat).forEach((entry)=>{

        });
        return (<div className='grid grid-cols-5 gap-4'>{byCat.map(e => (
            <div className="shadow-lg border-solid border-1 border-gray-600 rounded-lg"
                 key={e.id}
                 onClick={this.onClick.bind(this, e.id)}>{e.description}</div>
        ))}</div>)
    }

    renderLoading() {
        return (<div>Loading...</div>);
    }
}