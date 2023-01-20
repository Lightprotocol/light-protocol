{
  'targets': [{
    'target_name': 'addon',
    'sources': [
      './src/addon.cc',
      './src/blake224.c',
      './src/blake256.c',
      './src/blake384.c',
      './src/blake512.c'
    ],
    'include_dirs': [
      '<!@(node -p "require(\'node-addon-api\').include")',
    ],
    'cflags': [
      '-Wall',
      '-Wextra'
    ],
    'cflags!': [
      '-fno-exceptions',
    ],
    'cflags_cc!': [
      '-fno-exceptions',
    ],
    'defines': [
      'NAPI_VERSION=3',
    ],
    'xcode_settings': {
      'GCC_ENABLE_CPP_EXCEPTIONS': 'YES',
      'CLANG_CXX_LIBRARY': 'libc++',
      'MACOSX_DEPLOYMENT_TARGET': '10.7',
    },
    'msvs_settings': {
      'VCCLCompilerTool': {
        'ExceptionHandling': 1,
      },
    },
  }]
}
