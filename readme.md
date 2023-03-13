# Ping

The goal of this challenge is to create a flexible ICMP Echo implementation in
Rust.

## Why is this needed?

At Firezone, we often need low-level control over packet flows at layers 3 and
above of the OSI model.

Standard ping utilities on most OSes don't allow intervals below 100ms or
pinging multiple IPs at once, but these things are useful to Firezone Gateways
that quickly need to perform ping sweeps, for example.

## Phase 1 Requirements

Phase 1 of this challenge is to implement the solution that pings the targets
sequentially, in a single thread.

- You may assume this utility will only need to run on Linux
- You may use public crates like `pnet` as you wish to help you construct,
  deconstruct, and send packets, but be prepared to discuss the tradeoffs
  involved with your choice.
- Your implementation should allow for a minimum of 1 and a maximum of 500 IPv4s
  to be pinged.
- Each target IP will have a configurable ping interval, minimum: `1ms`,
  maximum: `1000ms`. Requests should be sent at this interval, regardless of
  when the reply is received.
- Each target will have a specified ping count, minimum: `1`, maximum: `10`.
- ICMP Echo timeout should be set to 5 seconds.
- The program should process pings for each passed in IP one at a time, i.e.
  **non-interleaved** output.
- The program should exit after all pings are sent and replies received or timed
  out.

### Input

Input will be passed in via STDIN as a CSV with the following format:

```
1.1.1.1,3,1000
```

Where the first column is the IPv4 address, the second column is the number of
requests to send, and the third column is the interval in milliseconds to send
the requests. You may assume there is no header row.

### Output

ICMP Echo Replies should be printed to STDOUT in the comma-delimited format:

```
1.1.1.1,0,54,7189
```

Where the fields are:
`IPv4,icmp_sequence_number,ttl,elapsed_time_in_microseconds`

### Phase 1 Example

```
$ echo "1.1.1.1,3,1000" | ping
1.1.1.1,0,54,7189
1.1.1.1,1,54,7750
1.1.1.1,2,54,6674
```

## Phase 2 Requirements

Phase 2 of this challenge is to now make the pinger asynchronous using the
[tokio async runtime](https://tokio.rs). Input and Output formats are the same.

Note that output will naturally be interleaved as multiple threads write to
STDOUT.

### Phase 2 Example

```
$ echo "1.1.1.1,3,1000\n8.8.8.8,1,100" | ping
1.1.1.1,0,54,7189
8.8.8.8,0,54,10123
1.1.1.1,1,54,7750
1.1.1.1,2,54,6674
```

## Bonus (Optional)

Add ICMPv6 support.
