import 'bootstrap/dist/css/bootstrap.css';
import React from 'react';
import './App.css';
import EventList from "./EventList";
import Api from './Api';

const api: Api = new Api();

interface State {}


class App extends React.Component<{}, State> {

    componentDidUpdate(prevProps: Readonly<{}>, prevState: Readonly<State>, snapshot?: any): void {
    }

    componentDidMount(): void {
    }

    handleEventSelect(eventId: string) {
    }

    render() {
        return (
            <div className="max-w-5xl justify-center">
                <EventList selected=""
                           onSelect={this.handleEventSelect.bind(this)}
                           loader={api.getEvents.bind(api)} />
            </div>
        );
    }
}

export default App;
