import events from "../globalEvents";

class Database {
    constructor(type) {
        this.type = type;

        this.load();
    }

    async load() {
        switch (this.type) {
            case "file":
                const importLow = await import("lowdb");
                const importFileSync = await import("lowdb/adapters/FileSync");

                const low = importLow.default;
                const FileSync = importFileSync.default;
                const adapter = new FileSync("./config/users.json");

                this.db = low(adapter);
                break;

            default:
                console.log("No database specified");
                break;
        }

        await this.importModules(this.type);
        events.on("db:request", this.requestHandler.bind(this));
        events.emit("db:connected");
    }

    async importModules(type) {
        try {
            let file = await import(`./${type}`);

            const functions = Object.entries(file);
            for (const [name, func] of functions) {
                Database.prototype[name] = func;
            }
        } catch (error) {
            console.log(error);
            console.log("Can't find specified database handler");
        }
    }

    requestHandler(id, request, ...optionalParams) {
        let response = this[request](...optionalParams);

        events.emit(id, {
            success: true,
            data: response
        });
    }
}

export default Database;
