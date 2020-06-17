import React from 'react';
import './App.css';
import EventList from "./EventList";
import Api, {Socket} from './Api';

const api: Api = new Api();
const socket: Socket = api.getSocket();

interface State {}


class App extends React.Component<{}, State> {

    componentDidMount(): void {
        socket.addListener((e) => {
            console.log("Event: ", e);
        });
    }

    handleEventSelect(eventId: string) {
    }

    render() {
        return (
            <div>
                <nav className="flex items-center justify-between flex-wrap bg-black p-6">
                    <div className="flex items-center flex-shrink-0 text-white mr-6">
                        <span className="font-semibold text-xl tracking-tight">waitmate</span>
                    </div>
                    <div className="block lg:hidden">
                        <button
                            className="flex items-center px-3 py-2 border rounded text-teal-200 border-teal-400 hover:text-white hover:border-white">
                            <svg className="fill-current h-3 w-3" viewBox="0 0 20 20" xmlns="http://www.w3.org/2000/svg">
                                <title>Menu</title>
                                <path d="M0 3h20v2H0V3zm0 6h20v2H0V9zm0 6h20v2H0v-2z"/>
                            </svg>
                        </button>
                    </div>
                </nav>

                <EventList selected=""
                           onSelect={this.handleEventSelect.bind(this)}
                           messenger={socket}
                           loader={api.getEvents.bind(api)} />
            </div>
        );
    }
}

export default App;
