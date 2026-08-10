import Secret from 'gi://Secret';

const SCHEMA_NAME = 'org.gnome.shell.extensions.langux.ApiKey';
const SERVICE_ATTRIBUTE = 'google-translation';
const ITEM_LABEL = 'Langux Google Translation API key';

const schema = new Secret.Schema(SCHEMA_NAME, Secret.SchemaFlags.NONE, {
    service: Secret.SchemaAttributeType.STRING,
});

const attributes = {service: SERVICE_ATTRIBUTE};

function callAsync(asyncFn, finishFn, ...args) {
    return new Promise((resolve, reject) => {
        args.push((_source, result) => {
            try {
                resolve(finishFn(result));
            } catch (error) {
                reject(error);
            }
        });
        asyncFn(...args);
    });
}

export const SecretStore = {
    async hasApiKey() {
        const password = await callAsync(
            Secret.password_lookup,
            Secret.password_lookup_finish,
            schema,
            attributes,
            null,
        );
        return password !== null;
    },

    async getApiKey() {
        return await callAsync(
            Secret.password_lookup,
            Secret.password_lookup_finish,
            schema,
            attributes,
            null,
        );
    },

    async saveApiKey(key) {
        return await callAsync(
            Secret.password_store,
            Secret.password_store_finish,
            schema,
            attributes,
            Secret.COLLECTION_DEFAULT,
            ITEM_LABEL,
            key,
            null,
        );
    },

    async deleteApiKey() {
        return await callAsync(
            Secret.password_clear,
            Secret.password_clear_finish,
            schema,
            attributes,
            null,
        );
    },
};
