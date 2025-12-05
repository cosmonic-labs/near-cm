# NEAR Wasm component model experiments

This repository contains a PoC showing unique features of component model, which could potentially benefit the NEAR project as well as a deserialization benchmark suite.

## Build and Run

```
$ cargo build --workspace --target wasm32-unknown-unknown --release --manifest-path ./contract/Cargo.toml
$ cargo build --workspace --target wasm32-unknown-unknown --release --manifest-path ./wasm-serde/Cargo.toml
$ cargo run ./contract/target/wasm32-unknown-unknown/release ./wasm-serde/target/wasm32-unknown-unknown/release
```

### Query

```
$ curl localhost:8080 -H "X-Contract: contract"
```

> myapp:app/custom@0.1.0#greet: func(s: string) -> string
>
> myapp:app/custom@0.1.0#add: func(a: u64, b: u64) -> u64
>
> myapp:app/custom@0.1.0#foo: func(t: record{foo: string, bar: string}) -> u64

### Invocation

```
$ curl localhost:8080 -H "X-Contract: contract" -H "X-Func: myapp:app/custom@0.1.0#greet" -H "X-Codec: wasm_serde_json" -d '["world"]'
```

> [String("Hello, world!")]


```
$ curl localhost:8080 -H "X-Contract: contract" -H "X-Func: myapp:app/custom@0.1.0#add" -H "X-Codec: wasm_serde_json" -d '[3, 5]'
```

> [U64(8)]


```
$ curl localhost:8080 -H "X-Contract: contract" -H "X-Func: myapp:app/custom@0.1.0#foo" -H "X-Codec: wasm_serde_json" -H "X-Target: mul" -d '[{"foo":"myfoo","bar":"mybar"}]'
```

Alternatively, use TOML:

```
curl localhost:8080 -H "X-Contract: contract" -H "X-Func: myapp:app/custom@0.1.0#foo" -H "X-Codec: wasm_serde_toml" -H "X-Target: mul" -d '[{ foo = "myfoo", bar = "mybar" }]'
```

> [U64(42)]

## Benchmarks

This repository contains Wasm module and Wasm component benchmarks with focus on JSON deserialization.

### Inputs

We use 2 different JSON inputs:

#### Small

```json
{"a": "test", "b": 42, "c": [0, 1, 2] }
```

#### Big

```json
{
  "signed": [
    {
      "payload": {
        "nonce": "XCkavXk45BCln15mDa50zMN+uWXqv6nVTFbY4vi3b9Y=",
        "message": "{\"signer_id\":\"1d3c4c1898200faa3273e06b1834098ec635c88e538aeceb095d18321861a970\",\"deadline\":\"2025-09-23T14:42:10.476Z\",\"intents\":[{\"intent\":\"token_diff\",\"diff\":{\"nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1\":\"999999000\",\"nep141:wrap.near\":\"-327580166752348350287445024\"}}]}",
        "recipient": "intents.near"
      },
      "standard": "nep413",
      "signature": "ed25519:Scq7yw8YPWEwni9Rvy8R9pEFUCmscUSkRAu2LB9grPcr6L1NoNELBtiZZ58wm1cDrsgWForeaADkHnVmaqE6ULP",
      "public_key": "ed25519:6Eb6wkNagkMg5EfZjo2AmStrUrxPKWLSHYDqVX7ofxtV"
    },
    {
      "payload": {
        "nonce": "7vX4hxEe9Hu5veUyivPWPxDHpYNsXzi7EG8bc05EIlA=",
        "message": "{\"signer_id\":\"foxboss.near\",\"deadline\":\"2025-09-23T14:42:10.476Z\",\"intents\":[{\"intent\":\"token_diff\",\"diff\":{\"nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1\":\"-1000000000\",\"nep141:wrap.near\":\"327579839172181597939094736\"}}]}",
        "recipient": "intents.near"
      },
      "standard": "nep413",
      "signature": "ed25519:5Md212LF1YcemtoGCUPfB9sv1pijraj2VkKrFJscawXA7XXuVg6hvWTPcAvz2CuBH8Ns16Rmik1n7r9ySyyQJqWY",
      "public_key": "ed25519:ESzYnUQTyVsNpk2u8ZHZ6q2MF8MHsuoKFRC4aqYAvcJD"
    }
  ]
}
```

This input is sourced from: https://pikespeak.ai/transaction-viewer/8rEAAvvj1SNB7fn7aczUo79k4niyNVtDWkm6FMyDWAUb

### Methodology

Wasmtime is configured using the same configuration as [currently used by `nearcore`](https://github.com/near/nearcore/blob/71aba039e747aa72939e8389558a19c1cfdd4dc3/runtime/near-vm-runner/src/wasmtime_runner/mod.rs#L384-L443), except for adding component model support.

#### Modules

We only measure a single Wasm module:

- `module bunding serde_json`: a module bundling `serde_json`, aiming to replicate a contract deployed on NEAR today.

Performance of this binary is used as a baseline that Wasm components are compared against.

For Wasm modules we benchmark 5 scenarios

##### `noop`

Instantiate module, call `noop`.

This benchmark represents the round-trip cost of invoking a function exported by a module.

##### `small input`

Instantiate module, call `run_small`.

Module acquires "small" input buffer by using `input_len` and `input` host functions.

##### `small input byte args`

Instantiate module, call `run_small_bytes`.

"Small" input buffer is copied to the module memory, pointer and length are passed to the module as function arguments.

##### `big input`

Instantiate module, call `run_big`.

Module acquires "big" input buffer by using `input_len` and `input` host functions.

##### `big input byte args`

Instantiate module, call `run_big_bytes`.

"Big" input buffer is copied to the module memory, pointer and length are passed to the module as function arguments.

#### Components

We measure two different Wasm components:

- `component bunding serde_json`: a component bundling `serde_json`, aiming to match existing module behavior as much as possible.
- `component composed with codec`: a component [composed](https://component-model.bytecodealliance.org/composing-and-distributing/composing.html), with a codec component implemented at [`./wasm-serde/json`](https://github.com/cosmonic-labs/wasm-serde/tree/56dc189630792cff8dae099275aa7659e331376e/json)

For Wasm components we benchmark 9 scenarios

##### `noop`

Instantiate component, call `noop`.

This benchmark represents the round-trip cost of invoking a function exported by a component.

##### `small input`

Instantiate component, call `run-small`.

Component acquires "small" input buffer by using `input` host function.

##### `small input byte args`

Instantiate component, call `run-small-bytes`.

"Small" input buffer is passed to the component as function argument.

##### `small input typed args`

Instantiate component, call `from-list` on the instantiated codec, call `run-small-typed`.

Costs of instantion of the codec and construction of the reflective type are not measured - it is assumed that in a real deployment, a pool of codec instances would be used and instances would be reused along the with the types.

A record corresponding to "small" input is passed to the component as function argument.

##### `small input deserialized typed args`

Instantiate component, call `run-small-typed`.

A record corresponding to "small" input is passed to the component as function argument.

This benchmark represents the cost of invoking `run-small-typed` without any deserialization.

##### `big input`

Instantiate component, call `run-big`.

Component acquires "big" input buffer by using `input` host function.

##### `big input byte args`

Instantiate component, call `run-big-bytes`.

"Big" input buffer is passed to the component as function argument.

##### `big input typed args`

Instantiate component, call `from-list` on the instantiated codec, call `run-big-typed`.

Costs of instantion of the codec and construction of the reflective type are not measured - it is assumed that in a real deployment, a pool of codec instances would be used and instances would be reused along the with the types.

A record corresponding to "big" input is passed to the component as function argument.

##### `big input deserialized typed args`

Instantiate component, call `run-big-typed`.

A record corresponding to "big" input is passed to the component as function argument.

This benchmark represents the cost of invoking `run-big-typed` without any deserialization.

### Results

Benchmarks were run on a machine provisioned for us by NEAR.

Generated `criterion` report for this benchmark can be found at [`./benches/criterion-report`](./benches/criterion-report).

<details>
    <summary><code>cargo bench</code></summary>

```
module bundling serde_json/noop
                        time:   [3.0822 µs 3.0851 µs 3.0882 µs]
Found 11 outliers among 100 measurements (11.00%)
  3 (3.00%) low severe
  2 (2.00%) low mild
  2 (2.00%) high mild
  4 (4.00%) high severe
module bundling serde_json/small input
                        time:   [20.307 µs 20.363 µs 20.423 µs]
Found 14 outliers among 100 measurements (14.00%)
  4 (4.00%) low severe
  2 (2.00%) low mild
  4 (4.00%) high mild
  4 (4.00%) high severe
module bundling serde_json/small input byte args
                        time:   [22.558 µs 22.628 µs 22.698 µs]
Found 10 outliers among 100 measurements (10.00%)
  3 (3.00%) low severe
  2 (2.00%) low mild
  2 (2.00%) high mild
  3 (3.00%) high severe
module bundling serde_json/big input
                        time:   [29.396 µs 29.574 µs 29.750 µs]
Found 7 outliers among 100 measurements (7.00%)
  3 (3.00%) low severe
  1 (1.00%) low mild
  3 (3.00%) high severe
module bundling serde_json/big input byte args
                        time:   [30.969 µs 31.062 µs 31.156 µs]
Found 10 outliers among 100 measurements (10.00%)
  5 (5.00%) low severe
  2 (2.00%) low mild
  1 (1.00%) high mild
  2 (2.00%) high severe

component bundling serde_json/noop
                        time:   [12.104 µs 12.118 µs 12.133 µs]
Found 16 outliers among 100 measurements (16.00%)
  2 (2.00%) low severe
  6 (6.00%) high mild
  8 (8.00%) high severe
component bundling serde_json/small input
                        time:   [25.992 µs 26.067 µs 26.142 µs]
Found 9 outliers among 100 measurements (9.00%)
  3 (3.00%) low severe
  3 (3.00%) low mild
  1 (1.00%) high mild
  2 (2.00%) high severe
component bundling serde_json/small input byte args
                        time:   [21.794 µs 21.842 µs 21.890 µs]
Found 10 outliers among 100 measurements (10.00%)
  2 (2.00%) low severe
  3 (3.00%) low mild
  3 (3.00%) high mild
  2 (2.00%) high severe
component bundling serde_json/small input typed args
                        time:   [37.909 µs 38.126 µs 38.370 µs]
Found 7 outliers among 100 measurements (7.00%)
  2 (2.00%) low severe
  2 (2.00%) low mild
  2 (2.00%) high mild
  1 (1.00%) high severe
component bundling serde_json/small input deserialized typed args
                        time:   [21.366 µs 21.470 µs 21.591 µs]
Found 6 outliers among 100 measurements (6.00%)
  2 (2.00%) low severe
  1 (1.00%) low mild
  1 (1.00%) high mild
  2 (2.00%) high severe
component bundling serde_json/big input
                        time:   [31.219 µs 31.316 µs 31.419 µs]
Found 12 outliers among 100 measurements (12.00%)
  3 (3.00%) low severe
  3 (3.00%) low mild
  3 (3.00%) high mild
  3 (3.00%) high severe
component bundling serde_json/big input byte args
                        time:   [30.693 µs 30.778 µs 30.869 µs]
Found 14 outliers among 100 measurements (14.00%)
  3 (3.00%) low severe
  1 (1.00%) low mild
  5 (5.00%) high mild
  5 (5.00%) high severe
component bundling serde_json/big input typed args
                        time:   [57.067 µs 57.253 µs 57.447 µs]
Found 11 outliers among 100 measurements (11.00%)
  3 (3.00%) low severe
  3 (3.00%) low mild
  2 (2.00%) high mild
  3 (3.00%) high severe
component bundling serde_json/big input deserialized typed args
                        time:   [26.933 µs 27.005 µs 27.081 µs]
Found 18 outliers among 100 measurements (18.00%)
  5 (5.00%) low severe
  5 (5.00%) low mild
  5 (5.00%) high mild
  3 (3.00%) high severe

component composed with codec/noop
                        time:   [21.563 µs 21.583 µs 21.607 µs]
Found 9 outliers among 100 measurements (9.00%)
  1 (1.00%) low severe
  2 (2.00%) low mild
  2 (2.00%) high mild
  4 (4.00%) high severe
component composed with codec/small input
                        time:   [53.704 µs 53.838 µs 53.976 µs]
Found 7 outliers among 100 measurements (7.00%)
  3 (3.00%) low severe
  1 (1.00%) low mild
  3 (3.00%) high severe
component composed with codec/small input byte args
                        time:   [53.201 µs 53.300 µs 53.404 µs]
Found 15 outliers among 100 measurements (15.00%)
  3 (3.00%) low severe
  6 (6.00%) low mild
  4 (4.00%) high mild
  2 (2.00%) high severe
component composed with codec/small input typed args
                        time:   [47.622 µs 47.748 µs 47.891 µs]
Found 9 outliers among 100 measurements (9.00%)
  1 (1.00%) low severe
  4 (4.00%) high mild
  4 (4.00%) high severe
component composed with codec/small input deserialized typed args
                        time:   [31.281 µs 31.323 µs 31.366 µs]
Found 10 outliers among 100 measurements (10.00%)
  4 (4.00%) low severe
  3 (3.00%) high mild
  3 (3.00%) high severe
component composed with codec/big input
                        time:   [70.464 µs 70.584 µs 70.703 µs]
Found 11 outliers among 100 measurements (11.00%)
  4 (4.00%) low severe
  1 (1.00%) low mild
  4 (4.00%) high mild
  2 (2.00%) high severe
component composed with codec/big input byte args
                        time:   [69.934 µs 70.067 µs 70.212 µs]
Found 9 outliers among 100 measurements (9.00%)
  3 (3.00%) low severe
  1 (1.00%) low mild
  2 (2.00%) high mild
  3 (3.00%) high severe
component composed with codec/big input typed args
                        time:   [62.434 µs 62.547 µs 62.668 µs]
Found 12 outliers among 100 measurements (12.00%)
  3 (3.00%) low severe
  4 (4.00%) low mild
  3 (3.00%) high mild
  2 (2.00%) high severe
component composed with codec/big input deserialized typed args
                        time:   [33.513 µs 33.550 µs 33.587 µs]
Found 7 outliers among 100 measurements (7.00%)
  3 (3.00%) low severe
  2 (2.00%) high mild
  2 (2.00%) high severe
```
</details>

<details>
    <summary><code>uname -a</code></summary>

```
Linux cosmonic-mainnet 6.8.0-1034-gcp #36~22.04.2-Ubuntu SMP Wed Aug  6 17:48:15 UTC 2025 x86_64 x86_64 x86_64 GNU/Linux
```
</details>

<details>
    <summary><code>free -mh</code></summary>

```
               total        used        free      shared  buff/cache   available
Mem:            62Gi       903Mi       3.7Gi       1.0Mi        58Gi        61Gi
Swap:             0B          0B          0B
```
</details>

<details>
    <summary><code>cat /proc/cpuinfo</code></summary>

```
processor	: 0
vendor_id	: AuthenticAMD
cpu family	: 25
model		: 1
model name	: AMD EPYC 7B13
stepping	: 0
microcode	: 0xffffffff
cpu MHz		: 2450.000
cache size	: 512 KB
physical id	: 0
siblings	: 16
core id		: 0
cpu cores	: 8
apicid		: 0
initial apicid	: 0
fpu		: yes
fpu_exception	: yes
cpuid level	: 13
wp		: yes
flags		: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdtscp lm constant_tsc rep_good nopl nonstop_tsc cpuid extd_apicid tsc_known_freq pni pclmulqdq ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm cmp_legacy cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw topoext ssbd ibrs ibpb stibp vmmcall fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 clzero xsaveerptr arat npt nrip_save umip vaes vpclmulqdq rdpid fsrm
bugs		: sysret_ss_attrs null_seg spectre_v1 spectre_v2 spec_store_bypass srso ibpb_no_ret
bogomips	: 4900.00
TLB size	: 2560 4K pages
clflush size	: 64
cache_alignment	: 64
address sizes	: 48 bits physical, 48 bits virtual
power management:

processor	: 1
vendor_id	: AuthenticAMD
cpu family	: 25
model		: 1
model name	: AMD EPYC 7B13
stepping	: 0
microcode	: 0xffffffff
cpu MHz		: 2450.000
cache size	: 512 KB
physical id	: 0
siblings	: 16
core id		: 1
cpu cores	: 8
apicid		: 2
initial apicid	: 2
fpu		: yes
fpu_exception	: yes
cpuid level	: 13
wp		: yes
flags		: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdtscp lm constant_tsc rep_good nopl nonstop_tsc cpuid extd_apicid tsc_known_freq pni pclmulqdq ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm cmp_legacy cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw topoext ssbd ibrs ibpb stibp vmmcall fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 clzero xsaveerptr arat npt nrip_save umip vaes vpclmulqdq rdpid fsrm
bugs		: sysret_ss_attrs null_seg spectre_v1 spectre_v2 spec_store_bypass srso ibpb_no_ret
bogomips	: 4900.00
TLB size	: 2560 4K pages
clflush size	: 64
cache_alignment	: 64
address sizes	: 48 bits physical, 48 bits virtual
power management:

processor	: 2
vendor_id	: AuthenticAMD
cpu family	: 25
model		: 1
model name	: AMD EPYC 7B13
stepping	: 0
microcode	: 0xffffffff
cpu MHz		: 2450.000
cache size	: 512 KB
physical id	: 0
siblings	: 16
core id		: 2
cpu cores	: 8
apicid		: 4
initial apicid	: 4
fpu		: yes
fpu_exception	: yes
cpuid level	: 13
wp		: yes
flags		: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdtscp lm constant_tsc rep_good nopl nonstop_tsc cpuid extd_apicid tsc_known_freq pni pclmulqdq ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm cmp_legacy cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw topoext ssbd ibrs ibpb stibp vmmcall fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 clzero xsaveerptr arat npt nrip_save umip vaes vpclmulqdq rdpid fsrm
bugs		: sysret_ss_attrs null_seg spectre_v1 spectre_v2 spec_store_bypass srso ibpb_no_ret
bogomips	: 4900.00
TLB size	: 2560 4K pages
clflush size	: 64
cache_alignment	: 64
address sizes	: 48 bits physical, 48 bits virtual
power management:

processor	: 3
vendor_id	: AuthenticAMD
cpu family	: 25
model		: 1
model name	: AMD EPYC 7B13
stepping	: 0
microcode	: 0xffffffff
cpu MHz		: 2450.000
cache size	: 512 KB
physical id	: 0
siblings	: 16
core id		: 3
cpu cores	: 8
apicid		: 6
initial apicid	: 6
fpu		: yes
fpu_exception	: yes
cpuid level	: 13
wp		: yes
flags		: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdtscp lm constant_tsc rep_good nopl nonstop_tsc cpuid extd_apicid tsc_known_freq pni pclmulqdq ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm cmp_legacy cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw topoext ssbd ibrs ibpb stibp vmmcall fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 clzero xsaveerptr arat npt nrip_save umip vaes vpclmulqdq rdpid fsrm
bugs		: sysret_ss_attrs null_seg spectre_v1 spectre_v2 spec_store_bypass srso ibpb_no_ret
bogomips	: 4900.00
TLB size	: 2560 4K pages
clflush size	: 64
cache_alignment	: 64
address sizes	: 48 bits physical, 48 bits virtual
power management:

processor	: 4
vendor_id	: AuthenticAMD
cpu family	: 25
model		: 1
model name	: AMD EPYC 7B13
stepping	: 0
microcode	: 0xffffffff
cpu MHz		: 2450.000
cache size	: 512 KB
physical id	: 0
siblings	: 16
core id		: 4
cpu cores	: 8
apicid		: 8
initial apicid	: 8
fpu		: yes
fpu_exception	: yes
cpuid level	: 13
wp		: yes
flags		: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdtscp lm constant_tsc rep_good nopl nonstop_tsc cpuid extd_apicid tsc_known_freq pni pclmulqdq ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm cmp_legacy cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw topoext ssbd ibrs ibpb stibp vmmcall fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 clzero xsaveerptr arat npt nrip_save umip vaes vpclmulqdq rdpid fsrm
bugs		: sysret_ss_attrs null_seg spectre_v1 spectre_v2 spec_store_bypass srso ibpb_no_ret
bogomips	: 4900.00
TLB size	: 2560 4K pages
clflush size	: 64
cache_alignment	: 64
address sizes	: 48 bits physical, 48 bits virtual
power management:

processor	: 5
vendor_id	: AuthenticAMD
cpu family	: 25
model		: 1
model name	: AMD EPYC 7B13
stepping	: 0
microcode	: 0xffffffff
cpu MHz		: 2450.000
cache size	: 512 KB
physical id	: 0
siblings	: 16
core id		: 5
cpu cores	: 8
apicid		: 10
initial apicid	: 10
fpu		: yes
fpu_exception	: yes
cpuid level	: 13
wp		: yes
flags		: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdtscp lm constant_tsc rep_good nopl nonstop_tsc cpuid extd_apicid tsc_known_freq pni pclmulqdq ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm cmp_legacy cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw topoext ssbd ibrs ibpb stibp vmmcall fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 clzero xsaveerptr arat npt nrip_save umip vaes vpclmulqdq rdpid fsrm
bugs		: sysret_ss_attrs null_seg spectre_v1 spectre_v2 spec_store_bypass srso ibpb_no_ret
bogomips	: 4900.00
TLB size	: 2560 4K pages
clflush size	: 64
cache_alignment	: 64
address sizes	: 48 bits physical, 48 bits virtual
power management:

processor	: 6
vendor_id	: AuthenticAMD
cpu family	: 25
model		: 1
model name	: AMD EPYC 7B13
stepping	: 0
microcode	: 0xffffffff
cpu MHz		: 2450.000
cache size	: 512 KB
physical id	: 0
siblings	: 16
core id		: 6
cpu cores	: 8
apicid		: 12
initial apicid	: 12
fpu		: yes
fpu_exception	: yes
cpuid level	: 13
wp		: yes
flags		: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdtscp lm constant_tsc rep_good nopl nonstop_tsc cpuid extd_apicid tsc_known_freq pni pclmulqdq ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm cmp_legacy cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw topoext ssbd ibrs ibpb stibp vmmcall fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 clzero xsaveerptr arat npt nrip_save umip vaes vpclmulqdq rdpid fsrm
bugs		: sysret_ss_attrs null_seg spectre_v1 spectre_v2 spec_store_bypass srso ibpb_no_ret
bogomips	: 4900.00
TLB size	: 2560 4K pages
clflush size	: 64
cache_alignment	: 64
address sizes	: 48 bits physical, 48 bits virtual
power management:

processor	: 7
vendor_id	: AuthenticAMD
cpu family	: 25
model		: 1
model name	: AMD EPYC 7B13
stepping	: 0
microcode	: 0xffffffff
cpu MHz		: 2450.000
cache size	: 512 KB
physical id	: 0
siblings	: 16
core id		: 7
cpu cores	: 8
apicid		: 14
initial apicid	: 14
fpu		: yes
fpu_exception	: yes
cpuid level	: 13
wp		: yes
flags		: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdtscp lm constant_tsc rep_good nopl nonstop_tsc cpuid extd_apicid tsc_known_freq pni pclmulqdq ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm cmp_legacy cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw topoext ssbd ibrs ibpb stibp vmmcall fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 clzero xsaveerptr arat npt nrip_save umip vaes vpclmulqdq rdpid fsrm
bugs		: sysret_ss_attrs null_seg spectre_v1 spectre_v2 spec_store_bypass srso ibpb_no_ret
bogomips	: 4900.00
TLB size	: 2560 4K pages
clflush size	: 64
cache_alignment	: 64
address sizes	: 48 bits physical, 48 bits virtual
power management:

processor	: 8
vendor_id	: AuthenticAMD
cpu family	: 25
model		: 1
model name	: AMD EPYC 7B13
stepping	: 0
microcode	: 0xffffffff
cpu MHz		: 2450.000
cache size	: 512 KB
physical id	: 0
siblings	: 16
core id		: 0
cpu cores	: 8
apicid		: 1
initial apicid	: 1
fpu		: yes
fpu_exception	: yes
cpuid level	: 13
wp		: yes
flags		: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdtscp lm constant_tsc rep_good nopl nonstop_tsc cpuid extd_apicid tsc_known_freq pni pclmulqdq ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm cmp_legacy cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw topoext ssbd ibrs ibpb stibp vmmcall fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 clzero xsaveerptr arat npt nrip_save umip vaes vpclmulqdq rdpid fsrm
bugs		: sysret_ss_attrs null_seg spectre_v1 spectre_v2 spec_store_bypass srso ibpb_no_ret
bogomips	: 4900.00
TLB size	: 2560 4K pages
clflush size	: 64
cache_alignment	: 64
address sizes	: 48 bits physical, 48 bits virtual
power management:

processor	: 9
vendor_id	: AuthenticAMD
cpu family	: 25
model		: 1
model name	: AMD EPYC 7B13
stepping	: 0
microcode	: 0xffffffff
cpu MHz		: 2450.000
cache size	: 512 KB
physical id	: 0
siblings	: 16
core id		: 1
cpu cores	: 8
apicid		: 3
initial apicid	: 3
fpu		: yes
fpu_exception	: yes
cpuid level	: 13
wp		: yes
flags		: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdtscp lm constant_tsc rep_good nopl nonstop_tsc cpuid extd_apicid tsc_known_freq pni pclmulqdq ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm cmp_legacy cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw topoext ssbd ibrs ibpb stibp vmmcall fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 clzero xsaveerptr arat npt nrip_save umip vaes vpclmulqdq rdpid fsrm
bugs		: sysret_ss_attrs null_seg spectre_v1 spectre_v2 spec_store_bypass srso ibpb_no_ret
bogomips	: 4900.00
TLB size	: 2560 4K pages
clflush size	: 64
cache_alignment	: 64
address sizes	: 48 bits physical, 48 bits virtual
power management:

processor	: 10
vendor_id	: AuthenticAMD
cpu family	: 25
model		: 1
model name	: AMD EPYC 7B13
stepping	: 0
microcode	: 0xffffffff
cpu MHz		: 2450.000
cache size	: 512 KB
physical id	: 0
siblings	: 16
core id		: 2
cpu cores	: 8
apicid		: 5
initial apicid	: 5
fpu		: yes
fpu_exception	: yes
cpuid level	: 13
wp		: yes
flags		: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdtscp lm constant_tsc rep_good nopl nonstop_tsc cpuid extd_apicid tsc_known_freq pni pclmulqdq ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm cmp_legacy cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw topoext ssbd ibrs ibpb stibp vmmcall fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 clzero xsaveerptr arat npt nrip_save umip vaes vpclmulqdq rdpid fsrm
bugs		: sysret_ss_attrs null_seg spectre_v1 spectre_v2 spec_store_bypass srso ibpb_no_ret
bogomips	: 4900.00
TLB size	: 2560 4K pages
clflush size	: 64
cache_alignment	: 64
address sizes	: 48 bits physical, 48 bits virtual
power management:

processor	: 11
vendor_id	: AuthenticAMD
cpu family	: 25
model		: 1
model name	: AMD EPYC 7B13
stepping	: 0
microcode	: 0xffffffff
cpu MHz		: 2450.000
cache size	: 512 KB
physical id	: 0
siblings	: 16
core id		: 3
cpu cores	: 8
apicid		: 7
initial apicid	: 7
fpu		: yes
fpu_exception	: yes
cpuid level	: 13
wp		: yes
flags		: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdtscp lm constant_tsc rep_good nopl nonstop_tsc cpuid extd_apicid tsc_known_freq pni pclmulqdq ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm cmp_legacy cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw topoext ssbd ibrs ibpb stibp vmmcall fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 clzero xsaveerptr arat npt nrip_save umip vaes vpclmulqdq rdpid fsrm
bugs		: sysret_ss_attrs null_seg spectre_v1 spectre_v2 spec_store_bypass srso ibpb_no_ret
bogomips	: 4900.00
TLB size	: 2560 4K pages
clflush size	: 64
cache_alignment	: 64
address sizes	: 48 bits physical, 48 bits virtual
power management:

processor	: 12
vendor_id	: AuthenticAMD
cpu family	: 25
model		: 1
model name	: AMD EPYC 7B13
stepping	: 0
microcode	: 0xffffffff
cpu MHz		: 2450.000
cache size	: 512 KB
physical id	: 0
siblings	: 16
core id		: 4
cpu cores	: 8
apicid		: 9
initial apicid	: 9
fpu		: yes
fpu_exception	: yes
cpuid level	: 13
wp		: yes
flags		: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdtscp lm constant_tsc rep_good nopl nonstop_tsc cpuid extd_apicid tsc_known_freq pni pclmulqdq ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm cmp_legacy cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw topoext ssbd ibrs ibpb stibp vmmcall fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 clzero xsaveerptr arat npt nrip_save umip vaes vpclmulqdq rdpid fsrm
bugs		: sysret_ss_attrs null_seg spectre_v1 spectre_v2 spec_store_bypass srso ibpb_no_ret
bogomips	: 4900.00
TLB size	: 2560 4K pages
clflush size	: 64
cache_alignment	: 64
address sizes	: 48 bits physical, 48 bits virtual
power management:

processor	: 13
vendor_id	: AuthenticAMD
cpu family	: 25
model		: 1
model name	: AMD EPYC 7B13
stepping	: 0
microcode	: 0xffffffff
cpu MHz		: 2450.000
cache size	: 512 KB
physical id	: 0
siblings	: 16
core id		: 5
cpu cores	: 8
apicid		: 11
initial apicid	: 11
fpu		: yes
fpu_exception	: yes
cpuid level	: 13
wp		: yes
flags		: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdtscp lm constant_tsc rep_good nopl nonstop_tsc cpuid extd_apicid tsc_known_freq pni pclmulqdq ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm cmp_legacy cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw topoext ssbd ibrs ibpb stibp vmmcall fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 clzero xsaveerptr arat npt nrip_save umip vaes vpclmulqdq rdpid fsrm
bugs		: sysret_ss_attrs null_seg spectre_v1 spectre_v2 spec_store_bypass srso ibpb_no_ret
bogomips	: 4900.00
TLB size	: 2560 4K pages
clflush size	: 64
cache_alignment	: 64
address sizes	: 48 bits physical, 48 bits virtual
power management:

processor	: 14
vendor_id	: AuthenticAMD
cpu family	: 25
model		: 1
model name	: AMD EPYC 7B13
stepping	: 0
microcode	: 0xffffffff
cpu MHz		: 2450.000
cache size	: 512 KB
physical id	: 0
siblings	: 16
core id		: 6
cpu cores	: 8
apicid		: 13
initial apicid	: 13
fpu		: yes
fpu_exception	: yes
cpuid level	: 13
wp		: yes
flags		: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdtscp lm constant_tsc rep_good nopl nonstop_tsc cpuid extd_apicid tsc_known_freq pni pclmulqdq ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm cmp_legacy cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw topoext ssbd ibrs ibpb stibp vmmcall fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 clzero xsaveerptr arat npt nrip_save umip vaes vpclmulqdq rdpid fsrm
bugs		: sysret_ss_attrs null_seg spectre_v1 spectre_v2 spec_store_bypass srso ibpb_no_ret
bogomips	: 4900.00
TLB size	: 2560 4K pages
clflush size	: 64
cache_alignment	: 64
address sizes	: 48 bits physical, 48 bits virtual
power management:

processor	: 15
vendor_id	: AuthenticAMD
cpu family	: 25
model		: 1
model name	: AMD EPYC 7B13
stepping	: 0
microcode	: 0xffffffff
cpu MHz		: 2450.000
cache size	: 512 KB
physical id	: 0
siblings	: 16
core id		: 7
cpu cores	: 8
apicid		: 15
initial apicid	: 15
fpu		: yes
fpu_exception	: yes
cpuid level	: 13
wp		: yes
flags		: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdtscp lm constant_tsc rep_good nopl nonstop_tsc cpuid extd_apicid tsc_known_freq pni pclmulqdq ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm cmp_legacy cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw topoext ssbd ibrs ibpb stibp vmmcall fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 clzero xsaveerptr arat npt nrip_save umip vaes vpclmulqdq rdpid fsrm
bugs		: sysret_ss_attrs null_seg spectre_v1 spectre_v2 spec_store_bypass srso ibpb_no_ret
bogomips	: 4900.00
TLB size	: 2560 4K pages
clflush size	: 64
cache_alignment	: 64
address sizes	: 48 bits physical, 48 bits virtual
power management:
```
</details>

### Conclusions

It is clear that component model today carries a performance penalty, which is significant for NEAR.

Bundling `serde_json` in the component provides near-identical performance with a payload, which is representative of the kind of payloads used by NEAR today.

Extracting the codec into a separate component, whether executed via component composition or via "runtime composition" handled by the host, unfortunately, carries a heavy performance penalty, at least today.

The "bundling" case, whether modules or components, relies on generated, optimized implemetation for deserialization of the buffer into the Rust struct. Using an external codec, we are forced to rely on dynamic implementation and simply cannot provide the same optimizations.

This benchmark suite was used to optimize the codec and drive the interface design, however there are still opportunities for optimizing the codec. It seems unlikely that an external codec approach would be able to match the performance of bundled `serde_json` with statically generated deserializer implementation in the foreseeable future.

An important observation is that component composition carries a significant performance penalty, which is apparent by looking at the `noop`, round-trip benchmark for the composed component case.
