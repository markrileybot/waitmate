import * as React from 'react';

export interface LoadableProps<T> {
    loader: () => Promise<T>,
}

export interface LoadableState<T> {
    data?: T,
    error: any,
    loading: boolean,
}

export abstract class LoadableComponent<T, P extends LoadableProps<T>, S extends LoadableState<T>> extends React.Component<P, S> {

    componentDidMount(): void {
        this.load();
    }

    load() {
        this.props.loader()
            .then(response => {
                this.componentDidLoad(response);
                this.setState({
                    data: response,
                    error: null,
                    loading: false
                });
            })
            .catch(error => {
                this.setState({
                    error: error.toString(),
                    loading: false
                });
            });
    }

    componentDidLoad(data: T) {}

    render() {
        if (this.state.loading) {
            return this.renderLoading();
        } else if (this.state.error) {
            return this.renderError();
        } else {
            return this.renderLoaded();
        }
    }

    renderLoading(): React.ReactNode {
        return (<div>Loading...</div>);
    }

    renderError(): React.ReactNode {
        return (<div>Error {this.state.error}</div>);
    }

    abstract renderLoaded(): React.ReactNode;
}