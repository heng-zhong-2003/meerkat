### a sketch for implementing Meerkat

## Tokio 
We employ Tokio lib to support both local communicates in local services and remote services.
Caveat: "Tokio provides many useful utilities. When writing asynchronous code, you cannot use the ordinary blocking APIs provided by the Rust standard library, and **must instead use asynchronous versions of them**"
Our project still (mostly) fits here as our semantics fits 
actor model (asynchronously processing "r" and "do e", as well
as Historiographer find and apply valid batch asyncly) instead
of Communicating Sequential Process. 
For locking, we need to think carefully about how to implement
the sequential behavior using Tokio's API

## Design overview
# Historiographer
the core algorithm supporting transactional style updates with strong consistency
repo link: https://github.com/invpt/historiographer/tree/rewrite
a more detail explanation: https://docs.google.com/document/d/1ZJ5H6IHJBxBnDHAPtDrUhaV1RwvFn9EKj_JvYJXHei4/edit 

# Live code update concurrently
here's a discussion on extension of Historiographer with code updates, which proposes the asynchronous model for code updates and triggered actions, plus the idea of maintaining version number 
https://docs.google.com/document/d/1sOW95mKMcjfaFfQNMB1xV0ffIfSD08WdYcWvsK2Slno

# Combine all together
- **Well behaved triggered actions**: Historiographer supports actions behaves in a transactional style with strong consistency
- **Well behaved code updates**: RLocks required on all ancestors, as well as all descendants of writes. WLocks are only required on writes. The reason we also require RLocks on descendants is we need to ensure code update "r" is well typed wrt *proper* typing environment. 
Open to discussion: which would be better for state var initialization
- upon gaining all locks, write new value to state var and propagate to subscribers
- or later we process this as an implicit triggered actions from developer
(*as a future step, we may design algorithms similar to Historiographer for this to avoid excessive RLocks, but keep in mind code update will not propagate like value update*) 
- **Well behaved code updates with triggered actions**: Locks ensure no partial 
code updates will be observed by user. Even though, we maintain version number for each state var and triggered action needs retypecheck only when version number changes

<!-- * If actions can only be stored in f, the using this mechanism, code updates will ensure all actions are well typed after updates. While developers holding the Locks, users can only see fully updated UI thus all triggered actions by users should be fine.
Or... we minimize the RLock on transitive writes? Or do not check actions well-type and defer the checking until user triggers? -->

## Version 1: implement a service 
Eventually we will partition the whole distributed program by
different services each connecting to different developers and 
users, both all should be capable of modifying any global states.
For now, it's a good starting point to implement one service:

# Service manager
struct to maintain:
- list of local developers
- list of local users
- map of local var/def actors and types
- map of local var/def actors and locks
- data structure of dependency graph for local state variable/definition
- *list of peer services* (not for now)
- *data structure of dependency graph for remote ...* (not for now)
- *random* queue R of code updates from local developer
- *random* queue E of triggered actions from local users
with implementing:
essentially sending messages as commands to local actors/global service managers
- create a new node and maintain its edges in dependency graph 
    - allocate a new actor
    - update subscriber sets for relevant nodes (local & remote)
    - **open to discussion**: 
        when receive a code update, and abstract its dependency graph,
        what is the exact behavior of updating the whole system,
        and data structure maintained at service manager? 
- delete a node as no longer used 
    - de-allocate an actor
    - update subscriber sets for relevant nodes (local & remote)
    - clear unapplied updates, and notify relevant developer/user
- take "r" and enqueue to R
    - static analysis of code snip "r" (any possibility to reject a "r" here?)
    (annotate type otherwise nonsense code like 1 + true may be enqueued)
- process code update "r"
    - require locks (locally in service and globally from others)
    - fail: abandon "r" when lock acquirement got rejected, 
        - notify developer
        - release lock
    - success: update local dependency graph, notify global services to update their dependency graph
        - create/delete relevant nodes
        - release locks upon success finished 
- take "do e" and enqueue to E 
    - static analysis of do action "do e"
- process "do e"
    - require locks
    - fail then abandon
    - success then update 
    - release locks

# Message types
- DevRLock
- DevWLock
- UsrRLock
- UsrWLock

# State variable actor
**struct to maintain**:
- actor
- subscribers, None(dependencies)
- value, type
*higrt repo combine below two into one data structure*
- waiting locks (allowed by Wait-Die criteria)
- held locks (one WLock or multiple RLock)
- ongoing writes (for not blocking on listening message)
*only for Historiographer Alg*
- appliedActionTxns
- requiredActionTxns (R set in <f:=v, P, R>)

**with implementing** 
(as an optimization, we can use two parallel threads/goroutine/etc. and try not to introduce race condition):
thread1>>
- a for loop casing on message types:
    * ideally DevLocks should always have priority over UsrLocks somehow (to prioritize initialized values over values written by actions, details open to discussion...)
    - lockAcquire: Wait-Die (with DevLock always > UsrLocks)
    - lockRelease: delete lock
    - usrReadRequest:
        - check permission (UsrR/WLock both ok)
        - update requiredActionTxns
        - grant read 
        - release Rlock 
    - usrWriteRequest:
        - check permission (UsrWLock)
        - update requiredActionTxns
        - grant pending write
    *DevLocks should similar to above*
    - devWriteRequest: 
    - devReadRequest:
thread2>>
- process pending (t, write): 
    - update appliedActionTxns
    - update requiredActionTxns
    - broadcast (f:=v, {(t, write)}, R) to subscribers
    - release WLock
- pick and grant next lock
    - with some priority here

# Definition actor 
**struct to maintain**:
- actor
- subscribers
- transReadVars

- exprFn (e as a function in f := e)
- replica (for recording latest arg values of exprFn)
- value (latest value of f)

*only for Historiographer Alg*
- changesToApply 
- changesApplied

**with implementing**:
- a for loop casing on message types:
    - lockAcquire (only from developer's update)
    - lockRelease
    - usrReadRequest
    - devReadRequest
    - devWriteRequest
    - changeMessage:
        - updates changesToApply
- searching for vaild batch and apply
- process pending writes

MORE TO BE FILLED ...

# Testing
race condition (not likely introduced by channels),
deadlocks (not likely introduced by actor model)
but still necessary to test