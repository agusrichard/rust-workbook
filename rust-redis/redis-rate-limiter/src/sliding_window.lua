-- Sliding window log, decided in one step.
-- Redis runs a script start to finish without running any other command in between,
-- so no other request can sneak in between the count and the write.
--
-- KEYS[1]  the client's log (sorted set: member = request id, score = timestamp in ms)
-- ARGV[1]  now, in ms
-- ARGV[2]  window, in ms
-- ARGV[3]  limit
-- ARGV[4]  unique id for this request
--
-- Returns {allowed (1/0), remaining, retry_after_ms}

local key = KEYS[1]
local now = tonumber(ARGV[1])
local window = tonumber(ARGV[2])
local limit = tonumber(ARGV[3])

-- Forget requests that slid out of the window
redis.call('ZREMRANGEBYSCORE', key, '-inf', now - window)
local count = redis.call('ZCARD', key)

if count >= limit then
  -- Blocked requests are not logged. A slot frees up when the oldest one slides out.
  local oldest = redis.call('ZRANGE', key, 0, 0, 'WITHSCORES')
  return {0, 0, tonumber(oldest[2]) + window - now}
end

-- Pass the timestamp on as the original string, so Lua never reformats a 13-digit number
redis.call('ZADD', key, ARGV[1], ARGV[4])
redis.call('PEXPIRE', key, window)
return {1, limit - count - 1, 0}
