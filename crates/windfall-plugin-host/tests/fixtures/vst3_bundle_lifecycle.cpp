// Independent native Mac ABI/ownership ledger, not the 21-class Rust fixture.
// Method order/layout: primary Steinberg pluginterfaces/base/{funknown,
// ipluginbase}.h; pinned primary interface provenance is in the proposal.
// No SDK implementation is copied or linked. No virtual destructor is added.
#include <CoreFoundation/CoreFoundation.h>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <type_traits>

static_assert(sizeof(Boolean) == 1 && std::is_unsigned<Boolean>::value);
static_assert(sizeof(bool) == 1 && sizeof(CFIndex) == sizeof(intptr_t));
static_assert(sizeof(CFTypeID) == sizeof(uintptr_t));
static_assert(sizeof(CFOptionFlags) == sizeof(uintptr_t));
static_assert(sizeof(CFPropertyListFormat) == sizeof(CFIndex));
static_assert(std::is_same<CFIndex, intptr_t>::value);
static_assert(std::is_same<CFTypeID, uintptr_t>::value);
static_assert(std::is_same<CFOptionFlags, uintptr_t>::value);
static_assert(std::is_same<CFStringEncoding, uint32_t>::value);
static_assert(std::is_same<UniChar, uint16_t>::value);
static_assert(std::is_same<decltype(&CFStringGetLength), CFIndex (*)(CFStringRef)>::value);
static_assert(std::is_same<decltype(&CFStringGetCharacterAtIndex),
    UniChar (*)(CFStringRef, CFIndex)>::value);
static_assert(std::is_same<decltype(&CFRelease), void (*)(CFTypeRef)>::value);
static_assert(std::is_same<decltype(&CFRetain), CFTypeRef (*)(CFTypeRef)>::value);
static_assert(std::is_same<decltype(&CFGetTypeID), CFTypeID (*)(CFTypeRef)>::value);
static_assert(std::is_same<decltype(&CFBundleGetTypeID), CFTypeID (*)(void)>::value);
static_assert(std::is_same<decltype(&CFDictionaryGetTypeID), CFTypeID (*)(void)>::value);
static_assert(std::is_same<decltype(&CFStringGetTypeID), CFTypeID (*)(void)>::value);
static_assert(std::is_same<decltype(&CFDataCreate),
    CFDataRef (*)(CFAllocatorRef, const UInt8*, CFIndex)>::value);
static_assert(std::is_same<decltype(&CFDictionaryGetValue),
    const void* (*)(CFDictionaryRef, const void*)>::value);
static_assert(std::is_same<decltype(&CFStringCreateWithCString),
    CFStringRef (*)(CFAllocatorRef, const char*, CFStringEncoding)>::value);
static_assert(std::is_same<decltype(&CFStringGetFileSystemRepresentation),
    Boolean (*)(CFStringRef, char*, CFIndex)>::value);
static_assert(std::is_same<decltype(&CFURLCreateFromFileSystemRepresentation),
    CFURLRef (*)(CFAllocatorRef, const UInt8*, CFIndex, Boolean)>::value);
static_assert(std::is_same<decltype(&CFURLGetFileSystemRepresentation),
    Boolean (*)(CFURLRef, Boolean, UInt8*, CFIndex)>::value);
static_assert(std::is_same<decltype(&CFBundleCreate),
    CFBundleRef (*)(CFAllocatorRef, CFURLRef)>::value);
static_assert(std::is_same<decltype(&CFBundleCopyExecutableURL),
    CFURLRef (*)(CFBundleRef)>::value);
static_assert(std::is_same<decltype(&CFBundleGetInfoDictionary),
    CFDictionaryRef (*)(CFBundleRef)>::value);
static_assert(std::is_same<decltype(&CFBundleIsExecutableLoaded),
    Boolean (*)(CFBundleRef)>::value);
static_assert(std::is_same<decltype(&CFBundleGetFunctionPointerForName),
    void* (*)(CFBundleRef, CFStringRef)>::value);
static_assert(std::is_same<decltype(&CFBundleLoadExecutableAndReturnError),
    Boolean (*)(CFBundleRef, CFErrorRef*)>::value);
static_assert(std::is_same<decltype(&CFPropertyListCreateWithData),
    CFPropertyListRef (*)(CFAllocatorRef, CFDataRef, CFOptionFlags,
                         CFPropertyListFormat*, CFErrorRef*)>::value);
struct FactoryInfo { char vendor[64], url[256], email[128]; int32_t flags; };
struct ClassInfo { char cid[16]; int32_t cardinality; char category[32], name[64]; };
static_assert(sizeof(FactoryInfo) == 452 && sizeof(ClassInfo) == 116);
struct Unknown {
    virtual int32_t queryInterface(const char*, void**) = 0;
    virtual uint32_t addRef() = 0;
    virtual uint32_t release() = 0;
};
struct PluginFactory : Unknown {
    virtual int32_t getFactoryInfo(FactoryInfo*) = 0;
    virtual int32_t countClasses() = 0;
    virtual int32_t getClassInfo(int32_t, ClassInfo*) = 0;
    virtual int32_t createInstance(const char*, const char*, void**) = 0;
};
static void event(char value) {
    FILE* file = std::fopen(LEDGER, "a");
    if (!file) std::abort();
    std::fputc(value, file);
    std::fclose(file);
}
static CFBundleRef refs[32] = {};
static unsigned entries = 0;
class Factory final : public PluginFactory {
    uint32_t references = 1;
public:
    int32_t queryInterface(const char* iid, void** output) override {
        // Canonical Mac FUID byte order, exact SDK IPluginFactory/FUnknown IDs.
        static const unsigned char factory[16] = {0x7A,0x4D,0x81,0x1C,0x52,0x11,0x4A,0x1F,0xAE,0xD9,0xD2,0xEE,0x0B,0x43,0xBF,0x9F};
        static const unsigned char unknown[16] = {0,0,0,0,0,0,0,0,0xC0,0,0,0,0,0,0,0x46};
        if (std::memcmp(iid, factory, 16) == 0 || std::memcmp(iid, unknown, 16) == 0) {
            *output = this; addRef(); return 0;
        }
        *output = nullptr; return -1;
    }
    uint32_t addRef() override { return ++references; }
    uint32_t release() override {
        if (!entries || !references) std::abort();
        if (--references) return references;
        event('F'); delete this; return 0;
    }
    int32_t getFactoryInfo(FactoryInfo* info) override { *info = {}; return 0; }
    int32_t countClasses() override { return 0; }
    int32_t getClassInfo(int32_t, ClassInfo*) override { return 1; }
    int32_t createInstance(const char*, const char*, void** output) override { *output = nullptr; return 1; }
};
__attribute__((constructor)) static void initialize() { event('I'); }
__attribute__((destructor)) static void deallocate() { event('U'); }
#ifndef OMIT_ENTRY
extern "C" __attribute__((visibility("default"))) bool bundleEntry(CFBundleRef ref) {
    if (!ref || CFGetTypeID(ref) != CFBundleGetTypeID() || entries == 32) std::abort();
    CFRetain(ref); refs[entries++] = ref; event('E');
#ifdef REENTER_ON_SECOND_ENTRY
    if (entries == 2) {
        // A C callback in this test executable, alive for the whole native call.
        reinterpret_cast<void (*)()>(static_cast<uintptr_t>(REENTRY_CALLBACK))();
    }
#endif
#ifdef REFUSE_ENTRY
    return false;
#else
    return true;
#endif
}
#endif
#ifndef OMIT_EXIT
extern "C" __attribute__((visibility("default"))) bool bundleExit() {
    if (!entries) std::abort();
    event('X'); CFRelease(refs[--entries]); refs[entries] = nullptr;
#ifdef REFUSE_EXIT
    return false;
#else
    return true;
#endif
}
#endif
#ifndef OMIT_FACTORY
extern "C" __attribute__((visibility("default"))) PluginFactory* GetPluginFactory() {
    if (!entries) std::abort();
    event('G');
#ifdef NULL_FACTORY
    return nullptr;
#else
    return new Factory;
#endif
}
#endif
