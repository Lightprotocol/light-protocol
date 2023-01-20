#include <napi.h>

extern "C" {
#include "blake.h"
}

#define CREATE_BLAKE_WRAPPER(size, out_size)                                   \
  class BlakeWrapper##size : public Napi::ObjectWrap<BlakeWrapper##size> {     \
   public:                                                                     \
    static Napi::Object Init(Napi::Env env) {                                  \
      Napi::Function func = DefineClass(                                       \
          env,                                                                 \
          "Blake" #size,                                                       \
          {                                                                    \
              InstanceMethod("update", &BlakeWrapper##size::Update),           \
              InstanceMethod("digest", &BlakeWrapper##size::Digest),           \
          });                                                                  \
      constructor = Napi::Persistent(func);                                    \
      constructor.SuppressDestruct();                                          \
      return func;                                                             \
    }                                                                          \
    BlakeWrapper##size(const Napi::CallbackInfo& info)                         \
        : Napi::ObjectWrap<BlakeWrapper##size>(info) {                         \
      blake##size##_init(&this->state);                                        \
    }                                                                          \
                                                                               \
   private:                                                                    \
    static Napi::FunctionReference constructor;                                \
    state##size state;                                                         \
    Napi::Value Update(const Napi::CallbackInfo& info) {                       \
      auto buf = info[0].As<Napi::Buffer<const unsigned char>>();              \
      blake##size##_update(&this->state, buf.Data(), buf.Length());            \
      return info.Env().Undefined();                                           \
    }                                                                          \
    Napi::Value Digest(const Napi::CallbackInfo& info) {                       \
      auto buf = Napi::Buffer<unsigned char>::New(info.Env(), out_size);       \
      blake##size##_final(&this->state, buf.Data());                           \
      return buf;                                                              \
    }                                                                          \
  };                                                                           \
  Napi::FunctionReference BlakeWrapper##size::constructor;

CREATE_BLAKE_WRAPPER(224, 28);
CREATE_BLAKE_WRAPPER(256, 32);
CREATE_BLAKE_WRAPPER(512, 64);
CREATE_BLAKE_WRAPPER(384, 48);

Napi::Object Init(Napi::Env env, Napi::Object exports) {
  exports.Set("Blake224", BlakeWrapper224::Init(env));
  exports.Set("Blake256", BlakeWrapper256::Init(env));
  exports.Set("Blake384", BlakeWrapper384::Init(env));
  exports.Set("Blake512", BlakeWrapper512::Init(env));
  return exports;
}

NODE_API_MODULE(NODE_GYP_MODULE_NAME, Init)
