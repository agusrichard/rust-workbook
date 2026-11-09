-- Puts a driver on the map (or moves them) and records the heartbeat, in one step.
-- KEYS[1] = geo key, KEYS[2] = seen key
-- ARGV[1] = longitude, ARGV[2] = latitude, ARGV[3] = driver, ARGV[4] = now in ms

-- If Redis rejects the coordinates, redis.call raises and the script stops here,
-- so a bad position never counts as a heartbeat. MULTI can't do this: it runs
-- the commands after a failing one anyway.
local added = redis.call('GEOADD', KEYS[1], ARGV[1], ARGV[2], ARGV[3])
redis.call('ZADD', KEYS[2], ARGV[4], ARGV[3])
return added
