#define _GNU_SOURCE
#define CL_TARGET_OPENCL_VERSION 300
#include <CL/cl.h>
#include <EGL/egl.h>
#include <X11/Xlib.h>
#include <X11/extensions/XInput2.h>
#include <X11/extensions/XTest.h>
#include <dlfcn.h>
#include <gbm.h>
#include <libinput.h>
#include <libudev.h>
#include <pipewire/pipewire.h>
#include <stdio.h>
#include <wayland-client.h>
#include <wayland-server.h>
#include <xcb/randr.h>
#include <xcb/render.h>
#include <xkbcommon/xkbcommon.h>

extern long __isoc23_strtol(const char *, char **, int);
extern void __cxa_call_terminate(void *);

/* Keep relocations for each native dependency without requiring a display. */
static void *volatile symbols[] = {
    (void *)wl_display_connect,
    (void *)wl_display_create,
    (void *)pw_get_library_version,
    (void *)eglGetDisplay,
    (void *)gbm_create_device,
    (void *)xcb_randr_query_version,
    (void *)xcb_render_query_version,
    (void *)XOpenDisplay,
    (void *)XIQueryVersion,
    (void *)XTestQueryExtension,
    (void *)libinput_path_create_context,
    (void *)udev_new,
    (void *)xkb_context_new,
    (void *)clGetPlatformIDs,
    (void *)__cxa_call_terminate,
};

int main(int argc, char **argv) {
    for (unsigned i = 0; i < sizeof(symbols) / sizeof(symbols[0]); i++) {
        if (symbols[i] == NULL) return 1;
    }
    char *end;
    if (__isoc23_strtol("123", &end, 10) != 123 || *end != '\0') return 1;

    /* PipeWire's build script loads libclang through bindgen. */
    if (argc > 1) {
        void *clang = dlopen(argv[1], RTLD_NOW);
        if (clang == NULL || dlsym(clang, "clang_createIndex") == NULL) {
            fprintf(stderr, "Cannot load libclang: %s\n", dlerror());
            return 1;
        }
        dlclose(clang);
    }
    puts("Native capture, input and ONNX compatibility dependencies passed.");
    return 0;
}
