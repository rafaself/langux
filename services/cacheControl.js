import Gio from 'gi://Gio';

export const CACHE_BUS_NAME = 'org.gnome.Shell.Extensions.Langux';
export const CACHE_OBJECT_PATH = '/org/gnome/Shell/Extensions/Langux';
export const CACHE_INTERFACE_NAME = 'org.gnome.Shell.Extensions.Langux.Cache';

export const CACHE_INTERFACE_XML = `
<node>
  <interface name="${CACHE_INTERFACE_NAME}">
    <method name="ClearCache"/>
  </interface>
</node>`;

export function createCacheControlObject(onClear) {
    return Gio.DBusExportedObject.wrapJSObject(CACHE_INTERFACE_XML, {
        ClearCache() {
            onClear();
        },
    });
}

export function clearCacheOverSessionBus() {
    const proxy = Gio.DBusProxy.new_for_bus_sync(
        Gio.BusType.SESSION,
        Gio.DBusProxyFlags.NONE,
        null,
        CACHE_BUS_NAME,
        CACHE_OBJECT_PATH,
        CACHE_INTERFACE_NAME,
        null,
    );
    proxy.call_sync('ClearCache', null, Gio.DBusCallFlags.NONE, 1000, null);
}
