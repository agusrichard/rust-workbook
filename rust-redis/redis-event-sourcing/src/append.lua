-- Appends one event to an account, but only if nobody else wrote to it first.
-- Redis runs a script start to finish without running any other command in between,
-- so the version check and the write happen as one step.
--
-- KEYS[1]  the account's event stream
-- KEYS[2]  the stream of every event from every account (what projections read)
-- ARGV[1]  the version the caller saw when it loaded the account, or 'any' to skip the check
-- ARGV[2]  account id
-- ARGV[3]  event type
-- ARGV[4]  amount
--
-- Returns {1, new_version} when appended, {0, current_version} on a conflict

-- The account stream is never trimmed, so its length is the number of events = the version
local version = redis.call('XLEN', KEYS[1])

if ARGV[1] ~= 'any' and tonumber(ARGV[1]) ~= version then
  return {0, version}
end

version = version + 1
redis.call('XADD', KEYS[1], '*', 'version', version, 'type', ARGV[3], 'amount', ARGV[4])
redis.call('XADD', KEYS[2], '*', 'account', ARGV[2], 'version', version, 'type', ARGV[3], 'amount', ARGV[4])
return {1, version}
