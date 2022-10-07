/*!
 * Transport is responsible for sending datagrams (in 4KB or smaller)
 * to the remote host end. 
 * 
 * Note that the APIs are not fixed, it's just the skeleton code.
 */
pub trait Transport { 

    /*!
        Resolve routing information received from the remote host.
        Function is similar as connect.
        
        TODO: args and return types are not fixed yet
     */
    fn resolve_routing_info() -> Result<(), ()>; 

    fn post_recvs();

    fn tx_burst() -> Result<(), ()>; 

    fn tx_flush() -> Result<(), ()>; 
}