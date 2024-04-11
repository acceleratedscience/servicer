from typing import List, Optional


class UserProvidedConfig:
    """
    UserProvidedConfig is a class that represents the service configuration

    :param name: the name of the service
    :param port: the port on which the service should running
    :param replicas: the number of replicas of the service
    :param cloud: the cloud on which the service should running
    """

    def __init__(self, name: str, port: int,
                 replicas: int, cloud: str) -> None: ...


class Dispatcher:
    """
    Dispatcher is a class that represents the service dispatcher, which is
    responsible for housing all the Servicing functionality
    """

    def __init__(self) -> None: ...

    def add_service(self, name: str,
                    config: Optional[UserProvidedConfig] = None) -> None:
        """
        Add a new service to the dispatcher

        :param name: the name of the service
        :param config: the configuration of the service
        """

    def remove_service(self, name: str) -> None:
        """
        Remove a service from the dispatcher

        :param name: the name of the service
        """

    def up(self, name: str) -> None:
        """
        Start a service

        :param name: the name of the service to start
        """

    def down(self, name: str) -> None:
        """
        Stop a service

        :param name: the name of the service to stop
        """

    def status(self, name: str, pretty: Optional[bool] = None) -> str:
        """
        Get the status of a service

        :param name: the name of the service
        :param pretty: whether to return the status in a pretty format
        :return: the status of the service in string format
        """

    def save(self) -> None:
        """
        Save the dispatcher's cache
        """

    def save_as_b64(self) -> str:
        """
        Save the dispatcher's cache as a base64 string

        :return: the base64 string of the cache
        """

    def load(self, name: str) -> None:
        """
        Load the dispatcher's cache

        :param name: the name of the service
        :return: the status of the service in string format
        """

    def load_as_b64(self, b64: str) -> None:
        """
        Load the dispatcher's cache from a base64 string

        :param b64: the base64 string of the cache
        """

    def list(self) -> List[str]:
        """
        List all the services

        :return: a list of all the services
        """
