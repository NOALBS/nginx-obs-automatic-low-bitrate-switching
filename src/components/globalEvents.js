import { EventEmitter } from "events";
import shortid from "shortid";

class Events extends EventEmitter {
    async do(event, fname, ...params) {
        return new Promise((resolve, reject) => {
            const id = shortid.generate();

            events.once(id, res => {
                if (res.success) {
                    resolve(res.data);
                } else {
                    reject(res.data);
                }
            });

            events.emit(event, id, fname, ...params);
        });
    }
}

let events = new Events();

export default events;
